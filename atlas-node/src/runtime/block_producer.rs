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
    last_proposed_height: std::sync::atomic::AtomicU64,
}

impl<P: P2pPublisher> BlockProducer<P> {
    pub fn new(cluster: Arc<Cluster>, p2p: P, mempool: Arc<Mempool>) -> Self {
        Self { 
            cluster, 
            p2p, 
            mempool,
            last_proposed_height: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Gossip pending transactions to ensure propagation
    pub async fn gossip_pending_txs(&self) {
        let txs = self.mempool.get_candidates(50).await.unwrap_or_default();
        for (_, tx) in txs {
             if let Ok(bytes) = serde_json::to_vec(&tx) {
                 self.p2p.publish("atlas/tx/v1", bytes).await.ok();
             }
        }
    }

    /// Attempt to produce a block if leader
    pub async fn try_produce_block(&self) -> Option<String> {
        // 1. Check if I am leader
        let leader_guard = self.cluster.current_leader.read().await;
        let local_node_id = self.cluster.local_node.read().await.id.clone();
        let am_i_leader = leader_guard.as_ref() == Some(&local_node_id);
        let am_i_leader = leader_guard.as_ref() == Some(&local_node_id);
        drop(leader_guard);

        tracing::info!("🕵️ [BlockProducer] Check: Leader? {} | Mempool Size: {}", am_i_leader, self.mempool.len().await.unwrap_or(0));

        if am_i_leader {
            // 2. Check Mempool
            // Force release stuck pending transactions (> 20s)
            let released = self.mempool.cleanup_pending(20).await.unwrap_or(0);
            if released > 0 {
                info!("🔓 Released {} zombie transactions back to mempool.", released);
            } else {
                tracing::info!("🔒 Pending Transactions: {}", self.mempool.pending_len().await.unwrap_or(0)); 
                // Need to expose pending_len helper or use read lock count 
            }

            if self.mempool.len().await.unwrap_or(0) > 0 {
                 info!("🔍 [BlockProducer] Leader checking mempool. Size: {}", self.mempool.len().await.unwrap_or(0));
            }
            let mut candidates = self.mempool.get_candidates(50).await.unwrap_or_default(); // BATCH_SIZE = 50
            
            // 2.0 State Validation: Filter out transactions already in Ledger
            // This prevents "Zombie" transactions (already committed) from blocking new blocks.
            let storage = self.cluster.local_env.storage.read().await;
            if let Some(ledger) = &storage.ledger {
                 let state = ledger.state.read().await;
                 
                 let initial_count = candidates.len();
                 let mut stale_hashes = Vec::new();

                 candidates.retain(|(hash, tx)| {
                     // Check nonce against ledger state
                     let sender = &tx.transaction.from;
                     let ledger_nonce = if let Some(acc) = state.accounts.get(sender) {
                         acc.nonce
                     } else if let Some(acc) = state.accounts.get(&format!("wallet:{}", sender)) {
                         acc.nonce
                     } else {
                         0
                     };

                     // tracing::error!("sender_nonce {} ledger_nonce {}",  tx.transaction.nonce, ledger_nonce);
                     
                     if tx.transaction.nonce <= ledger_nonce {
                         tracing::warn!("🗑️ Discarding Stale TX from sender {} (Nonce {} <= Current {})", sender, tx.transaction.nonce, ledger_nonce);
                         stale_hashes.push(hash.clone());
                         false
                     } else {
                         true
                     }
                 });
                 
                 if !stale_hashes.is_empty() {
                     info!("🧹 Cleaning up {} stale transactions from mempool.", stale_hashes.len());
                     // We must drop the state lock before calling mempool (though local mempool uses weird locking, better safe)
                     drop(state); 
                     self.mempool.remove_batch(&stale_hashes).await.ok();
                 } else {
                     drop(state);
                 }

                 if candidates.len() < initial_count {
                     info!("🧹 Filtered {} stale transactions from candidates.", initial_count - candidates.len());
                 }
            }
            drop(storage);

            // Deterministic Sort: Group by Sender, Order by Nonce
            candidates.sort_by(|a, b| {
                a.1.transaction.from.cmp(&b.1.transaction.from)
                    .then(a.1.transaction.nonce.cmp(&b.1.transaction.nonce))
            });
            
            if candidates.is_empty() {
                // Optimization: Do not flood network with empty blocks if no txs.
                tracing::info!("💤 Mempool empty, skipping block production.");
                return None;
            }

            info!("⛏️ Producing block with {} transactions", candidates.len());

            // 2.1 Throttling: Check if previous proposal is finalized
             let storage = self.cluster.local_env.storage.read().await;
             
             if let Some(last_prop) = storage.proposals.last() {
                 let is_committed = storage.results.get(&last_prop.id)
                     .map(|r| r.approved)
                     .unwrap_or(false);

                 if !is_committed {
                     // Still pending or rejected
                     info!("⏳ Waiting for consensus on proposal {} (Height {}). Skipping production.", last_prop.id, last_prop.height);
                     return None;
                 }
             }

             let current_chain_height = storage.proposals.len() as u64; 
             drop(storage);

             let target_height = current_chain_height + 1;
             
             // Check if we already proposed for this height
             let last = self.last_proposed_height.load(std::sync::atomic::Ordering::SeqCst);
             if target_height <= last {
                 info!("⏳ Waiting for consensus on Height {} (Last proposed: {}). Skipping proposal generation.", target_height, last);
                 // OPTIONAL: If it's been too long (timeout), we might want to re-broadcast or view change, 
                 // but bluntly creating a NEW proposal ID causes equivocation.
                 return None;
             }
                
                // 3. Serialize content as Vec<SignedTransaction>
                let txs: Vec<atlas_common::transactions::SignedTransaction> = candidates.iter().map(|(_, tx)| tx.clone()).collect();
                 let content = match serde_json::to_string(&txs) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to serialize transaction batch: {}", e);
                        return None;
                    }
                };

                // 4. Submit Proposal
                match self.submit_proposal(content).await {
                    Ok(pid) => {
                        info!("✅ Block Produced! Proposal ID: {}", pid);
                        self.last_proposed_height.store(target_height, std::sync::atomic::Ordering::SeqCst);
                        // 5. Mark as Pending (In-Flight)
                        if !candidates.is_empty() {
                            let hashes: Vec<String> = candidates.iter().map(|(h, _)| h.clone()).collect();
                            self.mempool.mark_pending(&hashes).await.ok();
                        }
                        return Some(pid);
                    },
                    Err(e) => {
                        error!("❌ Failed to produce block: {}", e);
                    }
                }
            // } <--- Removed closing brace for !candidates.is_empty()
        }
        None
    }

    pub async fn submit_proposal(&self, content: String) -> Result<String, String> {
        let id = format!("prop-{}", rand::random::<u64>());
        let local_node = self.cluster.local_node.read().await;
        let proposer = local_node.id.clone();
        let public_key = self.cluster.auth.read().await.public_key().to_vec();

        let storage = self.cluster.local_env.storage.read().await;
        
        // Dynamic Round Calculation:
        // Check if there are existing votes for this height (implying a failed/stalled view).
        // If so, increment the round to avoid "AlreadyVoted" equivocation errors.
        let engine = self.cluster.local_env.engine.lock().await;
        let max_view = engine.registry.get_highest_view();
        drop(engine);

        let round = match max_view {
            Some(v) => v + 1,
            None => 0,
        };

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
            round, 
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
