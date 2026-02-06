use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};
use tracing::info;
use atlas_p2p::{ports::P2pPublisher, events::AdapterEvent};
use atlas_consensus::cluster::core::Cluster;
use crate::rpc;
use atlas_mempool::Mempool;

use crate::runtime::consensus_driver::ConsensusDriver;
use crate::runtime::sync_driver::SyncDriver;
use crate::runtime::block_producer::BlockProducer;

pub struct Maestro<P: P2pPublisher> {
    pub cluster: Arc<Cluster>,
    pub p2p: P,
    pub mempool: Arc<Mempool>,
    pub evt_rx: Mutex<mpsc::Receiver<AdapterEvent>>,
    pub grpc_addr: SocketAddr,
    pub grpc_server_handle: Mutex<Option<JoinHandle<()>>>,
    
    // Drivers
    pub consensus: ConsensusDriver<P>,
    pub sync: SyncDriver<P>,
    pub block_producer: BlockProducer<P>,
}

impl<P: P2pPublisher + Clone + 'static> Maestro<P> {
    /// Submit external proposal (delegated to BlockProducer)
    pub async fn submit_external_proposal(&self, content: String) -> Result<String, String> {
        self.block_producer.submit_proposal(content).await
    }

    pub async fn get_status(&self) -> (String, String, u64, u64) {
        self.cluster.get_status().await
    }

    pub async fn run(self: Arc<Self>) {
        info!("[MAESTRO DEBUG] Maestro::run task started.");
        let mut election_timer = time::interval(Duration::from_secs(5));
        let mut sync_timer = time::interval(Duration::from_secs(10));
        let mut block_timer = time::interval(Duration::from_millis(2000));

        info!("[MAESTRO DEBUG] Entering main loop.");

        // Start gRPC server immediately
        {
            let grpc_addr_copy = self.grpc_addr;
            let maestro_clone = Arc::clone(&self);
            
            // Extract Ledger and Mempool
            let storage = self.cluster.local_env.storage.read().await;
            let ledger = storage.ledger.clone().expect("Ledger should be initialized");
            drop(storage);
            let mempool = Arc::clone(&self.mempool);

            let server_task = tokio::spawn(async move {
                if let Err(e) = rpc::server::run_server(maestro_clone, ledger, mempool, grpc_addr_copy).await {
                    eprintln!("gRPC Server Error: {}", e);
                }
            });
            let mut handle_guard = self.grpc_server_handle.lock().await;
            *handle_guard = Some(server_task);
            info!("gRPC Server started at {}", grpc_addr_copy);
        }

        loop {
            tokio::select! {
                res = self.evt_rx.lock() => {
                    let mut guard = res;
                    if let Some(evt) = guard.recv().await {
                        // Router: Dispatch event to appropriate driver
                        match evt {
                            AdapterEvent::Proposal(bytes) => {
                                self.consensus.handle_proposal(bytes).await;
                            },
                            
                            AdapterEvent::Vote(bytes) => {
                                self.consensus.handle_vote(bytes).await;
                            },

                            AdapterEvent::Evidence(bytes) => {
                                self.consensus.handle_evidence(bytes).await;
                            },
                            
                            AdapterEvent::TxRequest { from, req, req_id } => {
                                self.sync.handle_tx_request(from, req, req_id).await;
                            },

                            AdapterEvent::TxBundle { from, bundle } => {
                                self.sync.handle_tx_bundle(from, bundle).await;
                            },

                            AdapterEvent::Heartbeat{from, data} => {
                                info!("❤️ HB from {from} ({:?} bytes)", data.len());
                                tracing::debug!("❤️ HB from {from} ({:?} bytes)", data.len());
                                
                                // Update peer stats
                                let node = atlas_common::env::node::Node::new(from.clone(), "".to_string(), None, 0.0);
                                self.cluster.peer_manager.write().await.handle_command(
                                    atlas_p2p::peer_manager::PeerCommand::UpdateStats(from, node)
                                );
                            },

                            AdapterEvent::PeerDiscovered(id) => {
                                info!("🔍 Peer Discovered: {}", id);
                                let node = atlas_common::env::node::Node::new(id.clone(), "".to_string(), None, 0.0);
                                self.cluster.peer_manager.write().await.handle_command(
                                    atlas_p2p::peer_manager::PeerCommand::Register(id, node)
                                );
                            },

                            AdapterEvent::Gossip { topic, data, from } if topic == "atlas/heartbeat/v1" => {
                                tracing::info!("❤️ hb (fallback) from {from} ({} bytes)", data.len());
                            },

                            AdapterEvent::Gossip { topic, data, from: _ } if topic == "atlas/tx/v1" => {
                                // Mempool Ingest
                                if let Ok(tx) = serde_json::from_slice::<atlas_common::transactions::SignedTransaction>(&data) {
                                    use sha2::{Sha256, Digest};
                                    use atlas_common::transactions::signing_bytes;
                                    let mut hasher = Sha256::new();
                                    hasher.update(signing_bytes(&tx.transaction));
                                    hasher.update(&tx.signature);
                                    let hash = hex::encode(hasher.finalize());

                                    // Check Ledger for Idempotency AND Nonce Validity
                                    let (exists_in_ledger, is_valid_nonce, current_nonce) = {
                                        let storage = self.cluster.local_env.storage.read().await;
                                        if let Some(ledger) = &storage.ledger {
                                            let exists = ledger.exists_transaction(&hash).await.unwrap_or(false);
                                            
                                            // Nonce Check
                                            let state = ledger.state.read().await;
                                            let sender = &tx.transaction.from;
                                            let acc_nonce = if let Some(acc) = state.accounts.get(sender) {
                                                acc.nonce
                                            } else if let Some(acc) = state.accounts.get(&format!("wallet:{}", sender)) {
                                                acc.nonce
                                            } else {
                                                0
                                            };
                                            
                                            // Strict Nonce Check: Must be strictly +1
                                            // But for Gossip, we might receive out of order?
                                            // Actually, strict +1 is best for security. If we miss one, we request it via sync?
                                            // For now, let's just reject <= current (Replay) and allow gaps (Future)? 
                                            // User asked to "validate nonce".
                                            // Safe bet: > current. 
                                            let valid_nonce = tx.transaction.nonce > acc_nonce;
                                            
                                            (exists, valid_nonce, acc_nonce)
                                        } else {
                                            (false, true, 0)
                                        }
                                    };

                                    if exists_in_ledger {
                                        tracing::debug!("♻️ Transaction exists in Ledger. Ignoring gossip. Hash: {}", hash);
                                    } else if !is_valid_nonce {
                                        tracing::warn!("❌ Invalid Nonce via Gossip from {}. TxNonce: {} <= CurrentNonce: {}", tx.transaction.from, tx.transaction.nonce, current_nonce);
                                    } else {
                                        tracing::info!("📨 Received tx via Gossip! Hash: {} (Adding to Mempool)", hash);
                                        match self.mempool.add(tx).await {
                                            Ok(true) => info!("✅ Transaction added to Mempool (Gossip)"),
                                            Ok(false) => tracing::debug!("Duplicate transaction in Mempool (ignored)"),
                                            Err(e) => tracing::warn!("❌ Invalid transaction via Gossip: {}", e),
                                        }
                                    }
                                }
                            },

                            AdapterEvent::Gossip { topic, data, from: _ } if topic == "atlas/proposal/v1" => {
                                tracing::info!("📨 Received Proposal via Gossip");
                                self.consensus.handle_proposal(data).await;
                            },

                            AdapterEvent::Gossip { topic, data, from: _ } if topic == "atlas/vote/v1" => {
                                tracing::debug!("📨 Received Vote via Gossip");
                                self.consensus.handle_vote(data).await;
                            },

                            AdapterEvent::Gossip { topic, data, from: _ } if topic == "atlas/evidence/v1" => {
                                tracing::warn!("📨 Received Equivocation Evidence via Gossip");
                                self.consensus.handle_evidence(data).await;
                            },

                            AdapterEvent::Gossip { topic, .. } => {
                                tracing::debug!("Unhandled Gossip topic: {}", topic);
                            },

                            _ => {}
                        }
                    } else {
                        // Channel closed
                        break;
                    }
                },

                _ = election_timer.tick() => {
                    info!("[MAESTRO DEBUG] Election timer tick.");
                    self.cluster.elect_leader().await;

                    // Ensure Server is running
                    // (Same logic as before, checking gRPC handle)
                    let mut handle_guard = self.grpc_server_handle.lock().await;
                    let server_running = handle_guard.is_some();

                    if !server_running {
                        info!("gRPC Server not running. Restarting...");
                        let maestro_clone = Arc::clone(&self);
                        let grpc_addr_copy = self.grpc_addr;
                        
                        let ledger = self.cluster.local_env.storage.read().await.ledger.clone()
                            .expect("Ledger must be initialized");
                        let mempool = Arc::clone(&self.mempool);

                        let server_task = tokio::spawn(async move {
                            if let Err(e) = rpc::server::run_server(maestro_clone, ledger, mempool, grpc_addr_copy).await {
                                eprintln!("gRPC Server Error: {}", e);
                            }
                        });
                        *handle_guard = Some(server_task);
                    }
                },

                _ = sync_timer.tick() => {
                    // Periodic Sync Request
                    let peers = self.cluster.peer_manager.read().await.get_active_peers();
                    if let Some(node_id) = peers.iter().next() {
                        let my_height = self.cluster.local_env.storage.read().await.proposals.len() as u64;
                        if let Ok(peer) = node_id.0.parse::<libp2p::PeerId>() {
                            info!("🔄 Requesting sync from {} (my height: {})", peer, my_height);
                            if let Err(e) = self.p2p.request_state(peer, my_height).await {
                                eprintln!("Error requesting sync: {}", e);
                            }
                        }
                    }
                },

                _ = block_timer.tick() => {
                    // Gossip Pending Txs
                    self.block_producer.gossip_pending_txs().await;

                    // Block Production with Self-Voting
                    if let Some(pid) = self.block_producer.try_produce_block().await {
                         info!("🗳️ [Maestro] Leader Self-Voting for proposal {}", pid);
                         // 1. Create Vote
                         match self.cluster.create_vote(&pid, atlas_common::env::consensus::types::ConsensusPhase::Prepare).await {
                             Ok(Some(vote)) => {
                                 if let Ok(bytes) = bincode::serialize(&vote) {
                                     // 2. Broadcast (Gossip)
                                     self.p2p.publish("atlas/vote/v1", bytes.clone()).await.ok();
                                     // 3. Loopback (Local Processing)
                                     self.consensus.handle_vote(bytes).await;
                                     info!("✅ [Maestro] Self-Vote processed successfully");
                                 }
                             },
                             Ok(None) => tracing::warn!("⚠️ Failed to create self-vote (already voted or error)"),
                             Err(e) => tracing::error!("❌ Error creating self-vote: {}", e),
                         }
                    }
                }
            }
        }
    }
}
