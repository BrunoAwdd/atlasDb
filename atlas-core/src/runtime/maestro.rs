use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{self, Duration};
use tracing::info;
use crate::network::p2p::{ports::P2pPublisher, adapter::AdapterCmd, events::AdapterEvent};
use crate::cluster::core::Cluster;
use crate::rpc;


pub struct Maestro<P: P2pPublisher> {
    pub cluster: Arc<Cluster>,
    pub p2p: P,
    pub evt_rx: Mutex<mpsc::Receiver<AdapterEvent>>,
    pub grpc_addr: SocketAddr,
    pub grpc_server_handle: Mutex<Option<JoinHandle<()>>>,
}

use crate::env::proposal::Proposal;


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
            proposer,
            content,
            parent,
            height,
            hash: String::new(), 
            prev_hash,
            round: 0, // Placeholder
            time: chrono::Utc::now().timestamp(),
            state_root: String::new(), // Placeholder
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
            return Err(format!("Invalid signature length: {}", signature_vec.len()));
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
        
        // Start gRPC server immediately
        {
            let grpc_addr_copy = self.grpc_addr;
            let maestro_clone = Arc::clone(&self);
            let server_task = tokio::spawn(async move {
                if let Err(e) = rpc::server::run_server(maestro_clone, grpc_addr_copy).await {
                    eprintln!("Erro no servidor gRPC: {}", e);
                }
            });
            let mut handle_guard = self.grpc_server_handle.lock().await;
            *handle_guard = Some(server_task);
            info!("Servidor gRPC iniciado em {}", grpc_addr_copy);
        }

        let mut election_timer = time::interval(Duration::from_secs(5));
        let mut sync_timer = time::interval(Duration::from_secs(10));

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
                                            if let Err(e) = self.p2p.publish("atlas/vote/v1", bytes).await {
                                                eprintln!("Erro ao publicar voto Prepare: {}", e);
                                            }
                                        },
                                        Ok(None) => {},
                                        Err(e) => eprintln!("create_vote Prepare erro: {e}"),
                                    }
                                }
                            }
    
                            AdapterEvent::Vote(bytes) => {
                                if let Err(e) = self.cluster.handle_vote(bytes).await {
                                    eprintln!("handle_vote_bytes erro: {e}");
                                } else {
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
                                                                    self.p2p.publish("atlas/vote/v1", bytes).await.ok();
                                                                },
                                                                _ => {}
                                                            }
                                                        },
                                                        atlas_common::env::consensus::types::ConsensusPhase::PreCommit => {
                                                            // Quorum(PreCommit) -> Broadcast Commit
                                                            match self.cluster.create_vote(&result.proposal_id, atlas_common::env::consensus::types::ConsensusPhase::Commit).await {
                                                                Ok(Some(vote)) => {
                                                                    let bytes = bincode::serialize(&vote).unwrap();
                                                                    self.p2p.publish("atlas/vote/v1", bytes).await.ok();
                                                                },
                                                                _ => {}
                                                            }
                                                        },
                                                        atlas_common::env::consensus::types::ConsensusPhase::Commit => {
                                                            // Quorum(Commit) -> Finalize
                                                            info!("🎉 Proposta FINALIZADA (BFT): {}", result.proposal_id);
                                                            tracing::info!(target: "consensus", "EVENT:COMMIT id={} votes={}", result.proposal_id, result.votes_received);
                                                            
                                                            if let Err(e) = self.cluster.commit_proposal(result).await {
                                                                eprintln!("Erro ao commitar proposta: {}", e);
                                                            } else {
                                                                // Trigger election for the next block immediately
                                                                self.cluster.elect_leader().await;
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
                                let node = crate::cluster::node::Node::new(from.clone(), "".to_string(), None, 0.0);
                                self.cluster.peer_manager.write().await.handle_command(
                                    crate::peer_manager::PeerCommand::UpdateStats(from, node)
                                );
                            }

                            AdapterEvent::PeerDiscovered(id) => {
                                info!("🔍 Peer descoberto: {}", id);
                                let node = crate::cluster::node::Node::new(id.clone(), "".to_string(), None, 0.0);
                                self.cluster.peer_manager.write().await.handle_command(
                                    crate::peer_manager::PeerCommand::Register(id, node)
                                );
                            }
    
                            AdapterEvent::TxRequest { from, req, req_id } => {
                                match req {
                                    crate::network::p2p::protocol::TxRequest::GetState { height } => {
                                        info!("📥 Recebido pedido de estado de {} (altura > {})", from, height);
                                        let proposals = self.cluster.local_env.storage.read().await.get_proposals_after(height).await;
                                        let bundle = crate::network::p2p::protocol::TxBundle::State { proposals };
                                        
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
                                    crate::network::p2p::protocol::TxBundle::State { proposals } => {
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
                                                self.cluster.local_env.storage.write().await.log_proposal(p);
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
                            
    
                            _ => {}
                        }
                    } else {
                        // Canal fechado, encerrar o loop.
                        break;
                    }
                },

                _ = election_timer.tick() => {
                    self.cluster.elect_leader().await;
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
            }
        }
    }
}
