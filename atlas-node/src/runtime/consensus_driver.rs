use std::sync::Arc;
use tracing::{info, warn, error};
use atlas_p2p::ports::P2pPublisher;
use atlas_consensus::cluster::core::Cluster;
use atlas_common::env::consensus::types::ConsensusPhase;
use atlas_mempool::Mempool;

pub struct ConsensusDriver<P: P2pPublisher> {
    cluster: Arc<Cluster>,
    p2p: P,
    mempool: Arc<Mempool>,
}

impl<P: P2pPublisher> ConsensusDriver<P> {
    pub fn new(cluster: Arc<Cluster>, p2p: P, mempool: Arc<Mempool>) -> Self {
        Self { cluster, p2p, mempool }
    }

    pub async fn handle_proposal(&self, bytes: Vec<u8>) {
        if let Err(e) = self.cluster.handle_proposal(bytes.clone()).await {
            error!("handle_proposal_error: {e}");
            return;
        }

        // BFT Step 1: Receive Proposal -> Broadcast Prepare
        if let Ok(proposal) = bincode::deserialize::<atlas_common::env::proposal::Proposal>(&bytes) {
            match self.cluster.create_vote(&proposal.id, ConsensusPhase::Prepare).await {
                Ok(Some(vote)) => {
                    let vote_bytes = bincode::serialize(&vote).unwrap();
                    if let Err(e) = self.p2p.publish("atlas/vote/v1", vote_bytes.clone()).await {
                        error!("Failed to broadcast Prepare vote: {}", e);
                    }
                    
                    self.handle_vote(vote_bytes).await;
                },
                Ok(None) => {}, 
                Err(e) => error!("Failed to create Prepare vote: {}", e),
            }
        }
    }

    pub fn handle_vote(&self, bytes: Vec<u8>) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            let res = self.cluster.handle_vote(bytes.clone()).await;
            
            match res {
                Err(e) => error!("handle_vote error: {e}"),
                Ok(Some(evidence)) => {
                    warn!("🚨 Equivocation Detected via Vote! Broadcasting Evidence...");
                    if let Ok(ev_bytes) = bincode::serialize(&evidence) {
                        self.p2p.publish("atlas/evidence/v1", ev_bytes).await.ok();
                    }
                },
                Ok(None) => {
                    self.check_consensus_progress().await;
                }
            }
        })
    }

    pub async fn handle_evidence(&self, bytes: Vec<u8>) {
        if let Ok(evidence) = bincode::deserialize::<atlas_common::env::consensus::evidence::EquivocationEvidence>(&bytes) {
            info!("⚖️ Received EquivocationEvidence via P2P. Verifying...");
            match self.cluster.handle_evidence(evidence).await {
                Ok(_) => {}, 
                Err(e) => warn!("Evidence verification failed: {}", e),
            }
        }
    }

    async fn check_consensus_progress(&self) {
        match self.cluster.evaluate_proposals().await {
            Ok(results) => {
                for result in results {
                    if result.approved {
                        match result.phase {
                            ConsensusPhase::Prepare => {
                                self.broadcast_next_phase_vote(&result.proposal_id, ConsensusPhase::PreCommit).await;
                            },
                            ConsensusPhase::PreCommit => {
                                self.broadcast_next_phase_vote(&result.proposal_id, ConsensusPhase::Commit).await;
                            },
                            ConsensusPhase::Commit => {
                                info!("🎉 Proposal FINALIZED (BFT): {}", result.proposal_id);
                                tracing::info!(target: "consensus", "EVENT:COMMIT id={} votes={}", result.proposal_id, result.votes_received);
                                
                                if let Err(e) = self.cluster.commit_proposal(result.clone()).await {
                                    error!("Failed to commit proposal: {}", e);
                                } else {
                                    self.clean_mempool(&result.proposal_id).await;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => error!("evaluate_proposals error: {e}"),
        }
    }

    async fn broadcast_next_phase_vote(&self, proposal_id: &str, phase: ConsensusPhase) {
        match self.cluster.create_vote(proposal_id, phase.clone()).await {
            Ok(Some(vote)) => {
                let bytes = bincode::serialize(&vote).unwrap();
                self.p2p.publish("atlas/vote/v1", bytes.clone()).await.ok();
                self.handle_vote(bytes).await; 
            },
            Ok(None) => {},
            Err(e) => error!("Failed to create {:?} vote: {}", phase, e),
        }
    }

    async fn clean_mempool(&self, proposal_id: &str) {
         let storage = self.cluster.local_env.storage.read().await;
         if let Some(prop) = storage.proposals.iter().find(|p| p.id == proposal_id) {
             let tx_hashes: Vec<String> = if let Ok(batch) = serde_json::from_str::<Vec<atlas_common::transactions::SignedTransaction>>(&prop.content) {
                  use sha2::{Sha256, Digest};
                  use atlas_common::transactions::signing_bytes;
                  batch.iter().map(|tx| {
                      let mut hasher = Sha256::new();
                      hasher.update(signing_bytes(&tx.transaction));
                      hasher.update(&tx.signature);
                      hex::encode(hasher.finalize())
                  }).collect()
             } else if let Ok(tx) = serde_json::from_str::<atlas_common::transactions::SignedTransaction>(&prop.content) {
                   use sha2::{Sha256, Digest};
                   use atlas_common::transactions::signing_bytes;
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
