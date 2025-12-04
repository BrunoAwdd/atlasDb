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

        let mut proposal = Proposal {
            id,
            proposer,
            content,
            parent: None,
            signature: [0u8; 64],
            public_key,
        };

        // Use standardized signing bytes (bincode of ProposalSignView)
        let msg = atlas_sdk::env::proposal::signing_bytes(&proposal);
        let signature_vec = self.cluster.auth.read().await.sign(msg).map_err(|e| e.to_string())?;
        
        if signature_vec.len() == 64 {
            proposal.signature.copy_from_slice(&signature_vec);
            info!("✅ Proposta assinada com sucesso! ID: {}", proposal.id);
            tracing::info!(target: "consensus", "EVENT:PROPOSE id={} proposer={}", proposal.id, proposal.proposer);
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

    pub async fn run(self: Arc<Self>) {
        info!("[MAESTRO DEBUG] Tarefa Maestro::run iniciada.");
        let mut election_timer = time::interval(Duration::from_secs(5));

        info!("[MAESTRO DEBUG] Entrando no loop principal.");
        loop {
            tokio::select! {
                res = self.evt_rx.lock() => {
                    let mut guard = res;
                    if let Some(evt) = guard.recv().await {
                        // Processar o evento de rede
                        match evt {
                            AdapterEvent::Proposal(bytes) => {
                                if let Err(e) = self.cluster.handle_proposal(bytes).await {
                                    eprintln!("handle_proposal_bytes erro: {e}");
                                    continue;
                                }
                                match self.cluster.vote_proposals().await {
                                    Ok(votes) => {
                                        for vote in votes {
                                            let bytes = bincode::serialize(&vote).unwrap();
                                            if let Err(e) = self.p2p.publish("atlas/vote/v1", bytes).await {
                                                eprintln!("Erro ao publicar voto: {}", e);
                                            }
                                        }
                                    }
                                    Err(e) => eprintln!("vote_proposals erro: {e}"),
                                }
                            }
    
                            AdapterEvent::Vote(bytes) => {
                                if let Err(e) = self.cluster.handle_vote(bytes).await {
                                    eprintln!("handle_vote_bytes erro: {e}");
                                } else {
                                    // Check for consensus after receiving a vote
                                    match self.cluster.evaluate_proposals().await {
                                        Ok(results) => {
                                            for result in results {
                                                if result.approved {
                                                    info!("🎉 Proposta APROVADA: {}", result.proposal_id);
                                                    tracing::info!(target: "consensus", "EVENT:COMMIT id={} votes={}", result.proposal_id, result.votes_received);
                                                    
                                                    if let Err(e) = self.cluster.commit_proposal(result).await {
                                                        eprintln!("Erro ao commitar proposta: {}", e);
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

                    if am_i_leader && !server_running {
                        info!("Este nó é o líder. Iniciando servidor gRPC...");
                        let maestro_clone = Arc::clone(&self);
                        let server_task = tokio::spawn(async move {
                            if let Err(e) = rpc::server::run_server(maestro_clone, grpc_addr_copy).await {
                                eprintln!("Erro no servidor gRPC: {}", e);
                            }
                        });
                        *handle_guard = Some(server_task);
                    } else if !am_i_leader && server_running {
                        info!("Este nó não é mais o líder. Parando servidor gRPC...");
                        if let Some(task) = handle_guard.take() {
                            task.abort();
                        }
                    }
                }
            }
        }
    }
}
