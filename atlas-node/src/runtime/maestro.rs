use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};
use tracing::info;
use atlas_p2p::{ports::P2pPublisher, adapter::AdapterCmd, events::AdapterEvent};
use atlas_consensus::cluster::core::Cluster;
use crate::rpc;
// use atlas_ledger::state::State;
use atlas_common::crypto::merkle::calculate_merkle_root;
use atlas_mempool::Mempool;


pub struct Maestro<P: P2pPublisher> {
    pub cluster: Arc<Cluster>,
    pub p2p: P,
    pub mempool: Arc<Mempool>,
    pub evt_rx: Mutex<mpsc::Receiver<AdapterEvent>>,
    pub grpc_addr: SocketAddr,
    pub grpc_server_handle: Mutex<Option<JoinHandle<()>>>,
}

use atlas_common::env::proposal::Proposal;


impl<P: P2pPublisher + 'static> Maestro<P> {
    /// Cria e submete uma proposta vinda de uma fonte externa (ex: gRPC).
    pub async fn submit_external_proposal(&self, content: String) -> Result<String, String> {
        let id = format!("prop-{}", rand::random::<u64>());
        let local_node = self.cluster.local_node.read().await;
        let proposer = local_node.id.clone();
        let public_key = self.cluster.auth.read().await.public_key().to_vec();

        let storage = self.cluster.local_env.storage.read().await;
        
        let (parent, height, prev_hash) = if let Some(last_prop) = storage.proposals.last() {
            (Some(last_prop.id.clone()), last_prop.height + 1, last_prop.hash.clone())
        } else {
            (None, 1, "0000000000000000000000000000000000000000000000000000000000000000".to_string())
        };
        drop(storage); // Release lock

        let mut proposal = Proposal {
            id,
            proposer: proposer.clone(),
            content,
            parent,
            height,
            hash: String::new(), 
            prev_hash: prev_hash.clone(),
            round: 0, // Placeholder
            time: chrono::Utc::now().timestamp(),
            state_root: {
                // Manual construction of leaves for metadata
                use sha2::{Digest, Sha256};
                
                let mut leaves_map = std::collections::BTreeMap::new();
                leaves_map.insert("height", height.to_be_bytes().to_vec());
                leaves_map.insert("prev_hash", prev_hash.as_bytes().to_vec());
                leaves_map.insert("proposer", proposer.to_string().as_bytes().to_vec());

                let leaves: Vec<Vec<u8>> = leaves_map.iter().map(|(k, v)| {
                    let mut hasher = Sha256::new();
                    hasher.update(k.as_bytes());
                    hasher.update(v);
                    hasher.finalize().to_vec()
                }).collect();

                calculate_merkle_root(&leaves)
            },
            signature: [0u8; 64],
            public_key,
        };

        // Calculate hash
        proposal.hash = atlas_common::crypto::hash::compute_proposal_hash(&proposal);

        // Use standardized signing bytes (bincode of ProposalSignView)
        let msg = atlas_common::env::proposal::signing_bytes(&proposal);
        let signature_vec = self.cluster.auth.read().await.sign(msg).map_err(|e| e.to_string())?;
        
        if signature_vec.len() == 64 {
            proposal.signature.copy_from_slice(&signature_vec);
            info!("✅ Proposta assinada com sucesso! ID: {} Hash: {}", proposal.id, proposal.hash);
            tracing::info!(target: "consensus", "EVENT:PROPOSE id={} proposer={} hash={}", proposal.id, proposal.proposer, proposal.hash);
        } else {
            // TODO: Handle error properly
            panic!("Invalid signature length: {}", signature_vec.len());
        }
        
        let proposal_id = proposal.id.clone();

        // Chame o cluster para processar a proposta e retornar um comando de rede.
        let cmd = self.cluster.submit_proposal(proposal).await.map_err(|e| e.to_string())?;

        // Despache o comando para a camada de rede usando o publicador P2P.
        match cmd {
            AdapterCmd::Publish { topic, data } => {
                info!("Disseminando proposta externa via P2P...");
                self.p2p.publish(&topic, data).await.map_err(|e| e.to_string())?
            }
            _ => {
                return Err(
                    "Comando inesperado retornado de submit_proposal".to_string()
                );
            }
        }

        Ok(proposal_id)
    }

    pub async fn get_status(&self) -> (String, String, u64, u64) {
        self.cluster.get_status().await
    }

    pub async fn run(self: Arc<Self>) {
        info!("[MAESTRO DEBUG] Tarefa Maestro::run iniciada.");
        let mut election_timer = time::interval(Duration::from_secs(5));
        let mut sync_timer = time::interval(Duration::from_secs(10));
        let mut block_timer = time::interval(Duration::from_millis(2000));

        info!("[MAESTRO DEBUG] Entrando no loop principal.");

        // Start gRPC server immediately (Unconditionally)
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
                    eprintln!("Erro no servidor gRPC: {}", e);
                }
            });
            let mut handle_guard = self.grpc_server_handle.lock().await;
            *handle_guard = Some(server_task);
            info!("Servidor gRPC iniciado em {}", grpc_addr_copy);
        }

        loop {
            tokio::select! {
                res = self.evt_rx.lock() => {
                    let mut guard = res;
                    if let Some(evt) = guard.recv().await {
                        // Processar o evento de rede
                        match evt {
                            AdapterEvent::Proposal(bytes) => {
                                let bytes_clone = bytes.clone();
                                if let Err(e) = self.cluster.handle_proposal(bytes).await {
                                    eprintln!("handle_proposal_bytes erro: {e}");
                                    continue;
                                }
                                // BFT Step 1: Receive Proposal -> Broadcast Prepare
                                if let Ok(proposal) = bincode::deserialize::<atlas_common::env::proposal::Proposal>(&bytes_clone) {
                                     match self.cluster.create_vote(&proposal.id, atlas_common::env::consensus::types::ConsensusPhase::Prepare).await {
                                        Ok(Some(vote)) => {
                                            let bytes = bincode::serialize(&vote).unwrap();
                                            if let Err(e) = self.p2p.publish("atlas/vote/v1", bytes.clone()).await {
                                                eprintln!("Erro ao publicar voto Prepare: {}", e);
                                            }
                                            // BROADCAST EVIDENCE if detected
                                            match self.cluster.handle_vote(bytes).await {
                                                Ok(Some(evidence)) => {
                                                    warn!("🚨 Evidence DETECTED! Broadcasting to network...");
                                                    if let Ok(ev_bytes) = bincode::serialize(&evidence) {
                                                        self.p2p.publish("atlas/evidence/v1", ev_bytes).await.ok();
                                                    }
                                                },
                                                Ok(None) => {},
                                                Err(e) => eprintln!("Erro ao processar voto próprio (Prepare): {}", e),
                                            }
                                        },
                                        Ok(None) => {},
                                        Err(e) => eprintln!("create_vote Prepare erro: {e}"),
                                    }
                                }
                            }
    
                            AdapterEvent::Vote(bytes) => {
                                let res = self.cluster.handle_vote(bytes).await;
                                match res {
                                    Err(e) => eprintln!("handle_vote_bytes erro: {e}"),
                                    Ok(Some(evidence)) => {
                                         warn!("🚨 Equivocation Detected via P2P Vote! Broadcasting Evidence...");
                                         if let Ok(ev_bytes) = bincode::serialize(&evidence) {
                                             self.p2p.publish("atlas/evidence/v1", ev_bytes).await.ok();
                                         }
                                         // Still proceed to evaluate proposals? Yes, but offender slashed.
                                    },
                                    Ok(None) => {
                                        // Check for consensus progress
                                        match self.cluster.evaluate_proposals().await {
                                            Ok(results) => {
                                                for result in results {
                                                    if result.approved {
                                                        match result.phase {
                                                            atlas_common::env::consensus::types::ConsensusPhase::Prepare => {
                                                                // Quorum(Prepare) -> Broadcast PreCommit
                                                                match self.cluster.create_vote(&result.proposal_id, atlas_common::env::consensus::types::ConsensusPhase::PreCommit).await {
                                                                    Ok(Some(vote)) => {
                                                                        let bytes = bincode::serialize(&vote).unwrap();
                                                                        self.p2p.publish("atlas/vote/v1", bytes.clone()).await.ok();
                                                                        self.cluster.handle_vote(bytes).await.ok();
                                                                    },
                                                                    _ => {}
                                                                }
                                                            },
                                                            atlas_common::env::consensus::types::ConsensusPhase::PreCommit => {
                                                                // Quorum(PreCommit) -> Broadcast Commit
                                                                match self.cluster.create_vote(&result.proposal_id, atlas_common::env::consensus::types::ConsensusPhase::Commit).await {
                                                                    Ok(Some(vote)) => {
                                                                        let bytes = bincode::serialize(&vote).unwrap();
                                                                        self.p2p.publish("atlas/vote/v1", bytes.clone()).await.ok();
                                                                        self.cluster.handle_vote(bytes).await.ok();
                                                                    },
                                                                    _ => {}
                                                                }
                                                            },
                                                            atlas_common::env::consensus::types::ConsensusPhase::Commit => {

                                                            // Quorum(Commit) -> Finalize
                                                            info!("🎉 Proposta FINALIZADA (BFT): {}", result.proposal_id);
                                                            tracing::info!(target: "consensus", "EVENT:COMMIT id={} votes={}", result.proposal_id, result.votes_received);
                                                            
                                                            if let Err(e) = self.cluster.commit_proposal(result.clone()).await {
                                                                eprintln!("Erro ao commitar proposta: {}", e);
                                                            }

                                                            // Clean Mempool
                                                            {
                                                                let storage = self.cluster.local_env.storage.read().await;
                                                                if let Some(prop) = storage.proposals.iter().find(|p| p.id == result.proposal_id) {
                                                                    
                                                                    let tx_hashes: Vec<String> = if let Ok(batch) = serde_json::from_str::<Vec<atlas_common::transaction::SignedTransaction>>(&prop.content) {
                                                                         use sha2::{Sha256, Digest};
                                                                         use atlas_common::transaction::signing_bytes;
                                                                         batch.iter().map(|tx| {
                                                                             let mut hasher = Sha256::new();
                                                                             hasher.update(signing_bytes(&tx.transaction));
                                                                             hasher.update(&tx.signature);
                                                                             hex::encode(hasher.finalize())
                                                                         }).collect()
                                                                    } else if let Ok(tx) = serde_json::from_str::<atlas_common::transaction::SignedTransaction>(&prop.content) {
                                                                          // Re-calculate hash (same logic as Mempool)
                                                                          use sha2::{Sha256, Digest};
                                                                          use atlas_common::transaction::signing_bytes;
                                                                          let mut hasher = Sha256::new();
                                                                          hasher.update(signing_bytes(&tx.transaction));
                                                                          hasher.update(&tx.signature);
                                                                          vec![hex::encode(hasher.finalize())]
                                                                    } else {
                                                                        vec![]
                                                                    };

                                                                    if !tx_hashes.is_empty() {
                                                                        self.mempool.remove_batch(&tx_hashes);
                                                                        tracing::info!("🧹 Removed {} committed txs from mempool", tx_hashes.len());
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(e) => eprintln!("evaluate_proposals erro: {e}"),
                                    }
                                }
                            }
    
                            AdapterEvent::Heartbeat{from, data} => {
                                info!("❤️ HB de {from} ({:?} bytes)", data.len());
                                tracing::debug!("❤️ HB de {from} ({:?} bytes)", data.len());
                                
                                // Update peer stats
                                let node = atlas_common::env::node::Node::new(from.clone(), "".to_string(), None, 0.0);
                                self.cluster.peer_manager.write().await.handle_command(
                                    atlas_p2p::peer_manager::PeerCommand::UpdateStats(from, node)
                                );
                            }

                            AdapterEvent::PeerDiscovered(id) => {
                                info!("🔍 Peer descoberto: {}", id);
                                let node = atlas_common::env::node::Node::new(id.clone(), "".to_string(), None, 0.0);
                                self.cluster.peer_manager.write().await.handle_command(
                                    atlas_p2p::peer_manager::PeerCommand::Register(id, node)
                                );
                            }
    
                            AdapterEvent::TxRequest { from, req, req_id } => {
                                match req {
                                    atlas_p2p::protocol::TxRequest::GetState { height } => {
                                        info!("📥 Recebido pedido de estado de {} (altura > {})", from, height);
                                        let proposals = self.cluster.local_env.storage.read().await.get_proposals_after(height).await;
                                        let bundle = atlas_p2p::protocol::TxBundle::State { proposals };
                                        
                                        // Send response
                                        if let Err(e) = self.p2p.send_response(req_id, bundle).await {
                                            eprintln!("Erro ao enviar resposta de estado: {}", e);
                                        }
                                    },
                                    _ => {}
                                }
                            }

                            AdapterEvent::TxBundle { from, bundle } => {
                                match bundle {
                                    atlas_p2p::protocol::TxBundle::State { proposals } => {
                                        info!("📦 Recebido pacote de estado de {} com {} propostas", from, proposals.len());
                                        for p in proposals {
                                            // Validate and add to storage
                                            // TODO: Verify signatures? Yes, strictly we should.
                                            // For now, assume they are valid or rely on handle_proposal logic if reused.
                                            // But handle_proposal does gossip logic. Here we just want to store.
                                            
                                            // Verify signature
                                            let sign_bytes = atlas_common::env::proposal::signing_bytes(&p);
                                            let ok = self.cluster.auth.read().await
                                                .verify_with_key(sign_bytes, &p.signature, &p.public_key)
                                                .is_ok();

                                            if ok {
                                                self.cluster.local_env.storage.write().await.log_proposal(p).await;
                                            } else {
                                                tracing::warn!("❌ Assinatura inválida no State Transfer para proposta {}", p.id);
                                            }
                                        }
                                    },
                                    _ => {}
                                }
                            }

                            AdapterEvent::Gossip { topic, data, from } if topic == "atlas/heartbeat/v1" => {
                                tracing::info!("❤️ hb (fallback) de {from} ({} bytes)", data.len());
                            }

                            AdapterEvent::Gossip { topic, data, from: _ } if topic == "atlas/tx/v1" => {
                                if let Ok(tx) = serde_json::from_slice::<atlas_common::transaction::SignedTransaction>(&data) {
                                    use sha2::{Sha256, Digest};
                                    use atlas_common::transaction::signing_bytes;
                                    let mut hasher = Sha256::new();
                                    hasher.update(signing_bytes(&tx.transaction));
                                    hasher.update(&tx.signature);
                                    let hash = hex::encode(hasher.finalize());

                                    // Check Ledger for Idempotency
                                    let exists_in_ledger = {
                                        let storage = self.cluster.local_env.storage.read().await;
                                        if let Some(ledger) = &storage.ledger {
                                            ledger.exists_transaction(&hash).await.unwrap_or(false)
                                        } else {
                                            false
                                        }
                                    };

                                    if exists_in_ledger {
                                        tracing::debug!("♻️ Transação já existe no Ledger. Ignorando gossip. Hash: {}", hash);
                                    } else {
                                        tracing::info!("📨 Recebida transação via Gossip! Hash: {} (Adicionando ao Mempool)", hash);
                                        match self.mempool.add(tx) {
                                            Ok(true) => info!("✅ Transação adicionada ao Mempool (Gossip)"),
                                            Ok(false) => tracing::debug!("Transação duplicada no Mempool (ignorado)"),
                                            Err(e) => tracing::warn!("❌ Transação inválida via Gossip: {}", e),
                                        }
                                    }
                                }
                            }
                            

                            AdapterEvent::Evidence(bytes) => {
                                if let Ok(evidence) = bincode::deserialize::<atlas_common::env::consensus::evidence::EquivocationEvidence>(&bytes) {
                                     tracing::info!("⚖️ Recebida evidência de equivocação via P2P. Verificando...");
                                     match self.cluster.handle_evidence(evidence).await {
                                         Ok(_) => {}, 
                                         Err(e) => tracing::warn!("Erro ao processar evidência: {}", e),
                                     }
                                }
                            }
    
                            _ => {}
                        }
                    } else {
                        // Canal fechado, encerrar o loop.
                        break;
                    }
                },

                _ = election_timer.tick() => {
                    info!("[MAESTRO DEBUG] Timer da eleição disparou.");
                    self.cluster.elect_leader().await;

                    // Bloco para isolar os borrows e evitar conflitos de ownership
                    let (am_i_leader, grpc_addr_copy) = {
                        let leader_guard = self.cluster.current_leader.read().await;
                        let local_node_id = self.cluster.local_node.read().await.id.clone();
                        let am_i = leader_guard.as_ref() == Some(&local_node_id);
                        (am_i, self.grpc_addr) // Copia o endereço
                    };

                    let mut handle_guard = self.grpc_server_handle.lock().await;
                    let server_running = handle_guard.is_some();

                    info!("[MAESTRO DEBUG] Am I leader? {} | Server running? {}", am_i_leader, server_running);

                    if !server_running {
                        info!("Servidor gRPC não está rodando. Iniciando...");
                        let maestro_clone = Arc::clone(&self);
                        
                        // Extract Ledger and Mempool
                        let ledger = self.cluster.local_env.storage.read().await.ledger.clone()
                            .expect("Ledger must be initialized");
                        let mempool = Arc::clone(&self.mempool);

                        let server_task = tokio::spawn(async move {
                            if let Err(e) = rpc::server::run_server(maestro_clone, ledger, mempool, grpc_addr_copy).await {
                                eprintln!("Erro no servidor gRPC: {}", e);
                            }
                        });
                        *handle_guard = Some(server_task);
                    }
                }

                _ = sync_timer.tick() => {
                    let peers = self.cluster.peer_manager.read().await.get_active_peers();
                    // Pick a random peer (or just the first one for simplicity)
                    if let Some(node_id) = peers.iter().next() {
                        let my_height = self.cluster.local_env.storage.read().await.proposals.len() as u64;
                        if let Ok(peer) = node_id.0.parse::<libp2p::PeerId>() {
                            info!("🔄 Solicitando sync para {} (minha altura: {})", peer, my_height);
                            if let Err(e) = self.p2p.request_state(peer, my_height).await {
                                eprintln!("Erro ao solicitar sync: {}", e);
                            }
                        }
                    }
                }

                _ = block_timer.tick() => {
                    // Gossip Pending Transactions (Every 2s with block timer for simplicity, or separate)
                    // We simply broadcast everything in mempool to ensure leader gets it.
                    // Ideal: Diff mechanism. For now: Flood.
                    let txs = self.mempool.get_candidates(50); // Get up to 50 pending
                    for (_, tx) in txs {
                        // Serialize SignedTransaction
                         if let Ok(bytes) = serde_json::to_vec(&tx) {
                             self.p2p.publish("atlas/tx/v1", bytes).await.ok();
                         }
                    }

                    // Block Production Logic
                    // 1. Check if I am leader
                    let leader_guard = self.cluster.current_leader.read().await;
                    let local_node_id = self.cluster.local_node.read().await.id.clone();
                    let am_i_leader = leader_guard.as_ref() == Some(&local_node_id);
                    drop(leader_guard);

                    if am_i_leader {
                        // 2. Check Mempool
                        if self.mempool.len() > 0 {
                             tracing::info!("🔍 [Maestro] Leader checking mempool. Size: {}", self.mempool.len());
                        }
                        let candidates = self.mempool.get_candidates(50); // BATCH_SIZE = 50
                        if !candidates.is_empty() {

                            info!("⛏️ Producing block with {} transactions", candidates.len());
                            
                            // 3. Serialize content as Vec<SignedTransaction>
                            let txs: Vec<atlas_common::transaction::SignedTransaction> = candidates.iter().map(|(_, tx)| tx.clone()).collect();
                             let content = match serde_json::to_string(&txs) {
                                Ok(s) => s,
                                Err(e) => {
                                    tracing::error!("Failed to serialize transaction batch: {}", e);
                                    continue;
                                }
                            };

                            // 4. Submit Proposal
                            match self.submit_external_proposal(content).await {
                                Ok(pid) => {
                                    info!("✅ Block Produced! Proposal ID: {}", pid);
                                    // 5. Mark as Pending (In-Flight)
                                    let hashes: Vec<String> = candidates.iter().map(|(h, _)| h.clone()).collect();
                                    self.mempool.mark_pending(&hashes);
                                },
                                Err(e) => {
                                    tracing::error!("❌ Failed to produce block: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
