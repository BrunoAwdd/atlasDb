use std::sync::Arc;
use tracing::{info, error};
use atlas_p2p::{ports::P2pPublisher, adapter::AdapterCmd};
use atlas_consensus::cluster::core::Cluster;
use atlas_mempool::Mempool;
use atlas_common::env::proposal::Proposal;
use atlas_common::crypto::merkle::calculate_merkle_root;

pub struct BlockProducer<P: P2pPublisher> {
    cluster: Arc<Cluster>,
    p2p: P,
    mempool: Arc<Mempool>,
}

impl<P: P2pPublisher> BlockProducer<P> {
    pub fn new(cluster: Arc<Cluster>, p2p: P, mempool: Arc<Mempool>) -> Self {
        Self { cluster, p2p, mempool }
    }

    /// Gossip pending transactions to ensure propagation
    pub async fn gossip_pending_txs(&self) {
        let txs = self.mempool.get_candidates(50);
        for (_, tx) in txs {
             if let Ok(bytes) = serde_json::to_vec(&tx) {
                 self.p2p.publish("atlas/tx/v1", bytes).await.ok();
             }
        }
    }

    /// Attempt to produce a block if leader
    pub async fn try_produce_block(&self) {
        // 1. Check if I am leader
        let leader_guard = self.cluster.current_leader.read().await;
        let local_node_id = self.cluster.local_node.read().await.id.clone();
        let am_i_leader = leader_guard.as_ref() == Some(&local_node_id);
        drop(leader_guard);

        if am_i_leader {
            // 2. Check Mempool
            if self.mempool.len() > 0 {
                 info!("🔍 [BlockProducer] Leader checking mempool. Size: {}", self.mempool.len());
            }
            let candidates = self.mempool.get_candidates(50); // BATCH_SIZE = 50
            if !candidates.is_empty() {

                info!("⛏️ Producing block with {} transactions", candidates.len());
                
                // 3. Serialize content as Vec<SignedTransaction>
                let txs: Vec<atlas_common::transactions::SignedTransaction> = candidates.iter().map(|(_, tx)| tx.clone()).collect();
                 let content = match serde_json::to_string(&txs) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to serialize transaction batch: {}", e);
                        return;
                    }
                };

                // 4. Submit Proposal
                match self.submit_proposal(content).await {
                    Ok(pid) => {
                        info!("✅ Block Produced! Proposal ID: {}", pid);
                        // 5. Mark as Pending (In-Flight)
                        let hashes: Vec<String> = candidates.iter().map(|(h, _)| h.clone()).collect();
                        self.mempool.mark_pending(&hashes);
                    },
                    Err(e) => {
                        error!("❌ Failed to produce block: {}", e);
                    }
                }
            }
        }
    }

    pub async fn submit_proposal(&self, content: String) -> Result<String, String> {
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
            info!("✅ Proposal signed! ID: {} Hash: {}", proposal.id, proposal.hash);
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
                info!("Disseminating external proposal via P2P...");
                self.p2p.publish(&topic, data).await.map_err(|e| e.to_string())?
            }
            _ => {
                return Err(
                    "Unexpected command returned from submit_proposal".to_string()
                );
            }
        }

        Ok(proposal_id)
    }
}
