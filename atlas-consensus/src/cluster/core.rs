use std::sync::Arc;

use tokio::sync::{oneshot, Mutex, RwLock};
use tracing::info;
use atlas_common::{
    auth::Authenticator,
    utils::NodeId
};

use crate::env::runtime::AtlasEnv;
use atlas_p2p::PeerManager;
use super::node::Node;


// TODO: Implement retry logic for fail
// TODO: Implement periodic health checks
// TODO: make new tests
// TODO: Implemente new metrics

/// Simulates a distributed cluster composed of multiple nodes.
///
/// This structure provides mechanisms for broadcasting messages,
/// simulating inter-node communication, and running cyclical simulations.
pub struct Cluster {
    /// The full set of nodes currently part of the cluster.
    pub local_env: AtlasEnv,
    pub local_node: RwLock<Node>,
    pub peer_manager: Arc<RwLock<PeerManager>>,
    pub shutdown_sender: Mutex<Option<oneshot::Sender<()>>>,
    pub auth: Arc<RwLock<dyn Authenticator>>,
    pub current_leader: Arc<RwLock<Option<NodeId>>>,
}

impl Cluster {
    /// Initializes a new, empty cluster.
    pub fn new(
        env: AtlasEnv, 
        node_id: NodeId,
        auth: Arc<RwLock<dyn Authenticator>>,
    ) -> Self {
        let addr = "0.0.0.0:50052".to_string(); // Todo temp fix

        let peer_manager = Arc::clone(&env.peer_manager);
        
        Cluster {
            local_env: env,
            local_node: RwLock::new(Self::set_local_node(node_id, &addr)),
            peer_manager,
            shutdown_sender: Mutex::new(None),
            auth,
            current_leader: Arc::new(RwLock::new(None)),
        }
    }

    fn set_local_node(id: NodeId, addr: &str) -> Node {
        Node::new(id, addr.to_string(), None, 0.0)
    }



    pub async fn elect_leader(&self) {
        let peer_manager = self.peer_manager.read().await;
        // active_peers is HashSet<NodeId>
        let active_peers = peer_manager.get_active_peers();

        // Include self in the candidate list
        let local_node_id = self.local_node.read().await.id.clone();
        let mut candidates = active_peers;
        candidates.insert(local_node_id.clone());
        
        // If no candidates (should allow at least self?), return. 
        // But logic above handles it.

        info!("[ELECTION] Candidates: {:?}", candidates);
        
        // 1. Snapshot Stakes & Sort Deterministically
        let mut ranked_candidates: Vec<(NodeId, u64)> = Vec::new();
        let mut total_stake: u64 = 0;
        
        for node_id in candidates {
            let stake = self.get_validator_stake(&node_id).await;
            ranked_candidates.push((node_id, stake));
            total_stake += stake;
        }
        
        // Sort by NodeID to ensure deterministic order for the "Roulette"
        ranked_candidates.sort_by(|a, b| a.0.cmp(&b.0));
        
        if total_stake == 0 {
            // Fallback: Max ID (Genesis or all 0 stake)
            // Or better: Round Robin based on View?
            // For now, keep legacy Max ID if no stakes found (e.g. before Genesis applied or empty ledger).
            info!("⚠️ Total Stake is 0. Falling back to Max NodeID election.");
            let winner = ranked_candidates.last().unwrap().0.clone();
            self.update_leader(winner).await;
            return;
        }

        // 2. Generate Random Seed (Deterministic)
        // Source: Last Block Hash + View (Round)
        // This ensures that if the leader fails (View increases), a new leader is selected.
        let (seed_hash, _view) = {
            let storage = self.local_env.storage.read().await;
            let last_hash = storage.proposals.last().map(|p| p.hash.clone()).unwrap_or_else(|| "0000".to_string());
            let view = storage.proposals.last().map(|p| p.round).unwrap_or(0); 
            // Better: use current view from Consensus state, but Cluster doesn't track current view easily yet.
            // Using last_proposal view might be stale. 
            // Ideally we need `current_view` passed in or stored.
            // Let's assume `view` is passed or fetched.
            // For now, let's use `last_hash` as primary seed. 
            // To emulate view rotation, we could hash(last_hash + nonce)? 
            // Wait, `get_status` returns view. Let's rely on stored View if possible.
            // But `Cluster` doesn't control View. `Maestro` or `Consensus` does.
            // For this iteration, let's use `last_hash`.
            // RISK: If leader fails to produce, last_hash doesn't change -> Same leader elected -> Deadlock.
            // FIX: We MUST include a time-varying component or View.
            // Let's assume the caller (Maestro) manages View, but here we only have local storage.
            // PROVISIONAL: Hash(LastHash + SystemTime/Slot? No, must be deterministic).
            // Actually, `elect_leader` should accept `view` argument?
            // Existing signature is `&self`.
            // Let's stick to LastHash for Phase 3. 
            // Note: If leader halts, we are stuck until someone forces a view change (which creates a QC/Skip block?).
            // In typical BFT, View Change is explicit.
            (last_hash, view)
        };
        
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(seed_hash.as_bytes());
        // hasher.update(view.to_be_bytes()); // TODO: Inject View
        let result = hasher.finalize();
        
        // Convert first 8 bytes of hash to u64 for "Spin"
        let mut slice = [0u8; 8];
        slice.copy_from_slice(&result[0..8]);
        let spin_val = u64::from_be_bytes(slice);
        
        let winning_ticket = spin_val % total_stake;
        
        // 3. Select Winner
        let mut current_sum: u64 = 0;
        let mut winner: Option<NodeId> = None;
        
        for (id, stake) in ranked_candidates {
            current_sum += stake;
            if current_sum > winning_ticket {
                winner = Some(id);
                break;
            }
        }
        
        if let Some(w) = winner {
            info!("🎰 Election: Ticket {}/{} won by {:?} (Stake: {})", winning_ticket, total_stake, w, self.get_validator_stake(&w).await);
            self.update_leader(w).await;
        }
    }

    async fn update_leader(&self, new_leader: NodeId) {
        let mut current_leader_lock = self.current_leader.write().await;
        if *current_leader_lock != Some(new_leader.clone()) {
            info!("👑 Weighted Leader Elected: {:?}", new_leader);
            *current_leader_lock = Some(new_leader);
        }
    }

    /// Pure function for Weighted Lottery (Public for testing)
    pub fn weighted_lottery(candidates: &[(NodeId, u64)], total_stake: u64, seed_hash: &str) -> Option<NodeId> {
        if total_stake == 0 { return None; }
        
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(seed_hash.as_bytes());
        let result = hasher.finalize();
        
        let mut slice = [0u8; 8];
        slice.copy_from_slice(&result[0..8]);
        let spin_val = u64::from_be_bytes(slice);
        
        let winning_ticket = spin_val % total_stake;
        
        let mut current_sum: u64 = 0;
        for (id, stake) in candidates {
            current_sum += stake;
            if current_sum > winning_ticket {
                return Some(id.clone());
            }
        }
        None
    }

    pub async fn get_status(&self) -> (String, String, u64, u64) {
        let node_id = self.local_node.read().await.id.0.clone();
        
        let leader_id = self.current_leader.read().await.clone()
            .map(|id| id.0)
            .unwrap_or("".to_string());

        let storage = self.local_env.storage.read().await;
        let last_proposal = storage.proposals.last();
        
        let (height, view) = if let Some(p) = last_proposal {
            (p.height, p.round)
        } else {
            (0, 0)
        };

        (node_id, leader_id, height, view)
    }

    /// Returns the stake (ATLAS balance) for a given validator NodeId.
    /// Converts PeerId -> Address -> Ledger Balance.
    pub async fn get_validator_stake(&self, node_id: &NodeId) -> u64 {
        // 1. Convert PeerId to Address
        let address = match self.node_id_to_address(&node_id.0) {
            Some(addr) => addr,
            None => {
                tracing::warn!("⚠️ Generic Validator Stake Error: Could not derive address from NodeId {}", node_id.0);
                return 0; // Default to 0 stake (no voting power)
            }
        };

        // 2. Query Ledger
        // We need to access ledger from storage.
        // Storage lock might differ. Storage is RwLock.
        {
            let storage = self.local_env.storage.read().await;
            if let Some(ledger) = &storage.ledger {
                match ledger.get_balance(&address, "ATLAS").await {
                    Ok(bal) => return bal,
                    Err(e) => {
                         tracing::warn!("⚠️ Failed to query ledger balance for {}: {}", address, e);
                         return 0;
                    }
                }
            }
        }
        
        0
    }

    /// Helper to convert a Libp2p PeerId string into an Atlas Base58 Address.
    /// Assumes Ed25519 Identity Keys.
    fn node_id_to_address(&self, node_id_str: &str) -> Option<String> {
        // We use libp2p dependency to parse
        use std::str::FromStr;
        let peer_id = libp2p::PeerId::from_str(node_id_str).ok()?;
        let bytes = peer_id.to_bytes();

        // Check for Ed25519 Identity Key pattern:
        // 0x00 (Identity Code)
        // 0x24 (Length 36)
        // 0x08 0x01 (KeyType Ed25519)
        // 0x12 0x20 (Field Data, Length 32)
        // Total prefix: 6 bytes [0, 36, 8, 1, 18, 32]
        if bytes.len() == 38 && bytes.starts_with(&[0x00, 0x24, 0x08, 0x01, 0x12, 0x20]) {
             let pub_key_bytes = &bytes[6..];
             return Some(bs58::encode(pub_key_bytes).into_string());
        }

        tracing::warn!("NodeId {} does not match expected Ed25519 Identity pattern.", node_id_str);
        None
    }

    /// Handles external evidence received via Gossip.
    /// Verifies signatures and slashes the offender if valid.
    pub async fn handle_evidence(&self, evidence: atlas_common::env::consensus::evidence::EquivocationEvidence) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use atlas_common::env::vote_data::vote_signing_bytes;
        
        let offender = evidence.vote_a.voter.clone();
        
        // 1. Verify Offender Identity (Votes must be from same offender)
        if evidence.vote_b.voter != offender {
             tracing::warn!("❌ Invalid Evidence: Voters differ ({} vs {})", offender, evidence.vote_b.voter);
             return Ok(());
        }

        // 2. Verify Signatures of both votes
        let auth = self.auth.read().await;
        
        let sign_a = vote_signing_bytes(&evidence.vote_a);
        if let Err(e) = auth.verify_with_key(sign_a, &evidence.vote_a.signature, &evidence.vote_a.public_key) {
             tracing::warn!("❌ Invalid Evidence: Vote A signature invalid: {}", e);
             return Ok(());
        }

        let sign_b = vote_signing_bytes(&evidence.vote_b);
        if let Err(e) = auth.verify_with_key(sign_b, &evidence.vote_b.signature, &evidence.vote_b.public_key) {
             tracing::warn!("❌ Invalid Evidence: Vote B signature invalid: {}", e);
             return Ok(());
        }
        
        // 3. Verify Equivocation Logic (Same View, Same Phase, Conflict)
        if evidence.vote_a.view != evidence.vote_b.view || evidence.vote_a.phase != evidence.vote_b.phase {
             tracing::warn!("❌ Invalid Evidence: View/Phase mismatch");
             return Ok(());
        }
        
        let conflict = evidence.vote_a.proposal_id != evidence.vote_b.proposal_id || evidence.vote_a.vote != evidence.vote_b.vote;
        if !conflict {
             tracing::warn!("❌ Invalid Evidence: Votes are identical (No conflict)");
             return Ok(());
        }

        // 4. SLASHING
        tracing::info!("⚔️ VALID EVIDENCE RECEIVED! Slashing validator {}...", offender);
        
        // Convert NodeId to Address
        if let Some(address) = self.node_id_to_address(&offender.0) {
             let storage = self.local_env.storage.read().await;
             if let Some(ledger) = &storage.ledger {
                 if let Err(e) = ledger.slash_validator(&address, 1_000_000).await {
                      tracing::warn!("❌ Failed to slash validator {}: {}", address, e);
                 } else {
                      tracing::info!("💀 Validator {} successfully slashed via Evidence!", address);
                 }
             }
        } else {
             tracing::warn!("⚠️ Cannot slash node {}: Address conversion failed.", offender.0);
        }

        Ok(()) 
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_id_to_address_logic() {
        // Node 1 (Example)
        let node_id_str = "12D3KooWQJX75u9CGtL8vT6P6NMZr5azqHzcNKWQDAeA39d9P6Ks";
        let expected_addr = "FV9wLmZV5z4eWZaxTmcE5HWALxwyLdRvFaH8fAUFV9bw";
        
        use std::str::FromStr;
        if let Ok(peer_id) = libp2p::PeerId::from_str(node_id_str) {
             let bytes = peer_id.to_bytes();
             if bytes.len() == 38 && bytes.starts_with(&[0x00, 0x24, 0x08, 0x01, 0x12, 0x20]) {
                 let pub_key_bytes = &bytes[6..];
                 let addr = bs58::encode(pub_key_bytes).into_string();
                 assert_eq!(addr, expected_addr);
             } else {
                panic!("Pattern match failed for known valid ID");
            }
        } else {
            panic!("Failed to parse PeerId from string");
        }
    }

    #[test]
    fn test_weighted_election_distribution() {
        use atlas_common::utils::NodeId;
        
        let candidates = vec![
            (NodeId("Sardine".to_string()), 10), // 10%
            (NodeId("Tuna".to_string()), 30),    // 30%
            (NodeId("Whale".to_string()), 60),   // 60%
        ];
        let total = 100;
        
        let mut wins = std::collections::HashMap::new();
        wins.insert("Sardine", 0);
        wins.insert("Tuna", 0);
        wins.insert("Whale", 0);
        
        // Run 1000 elections with different seeds
        for i in 0..1000 {
            let seed = format!("block-{}", i);
            let winner = Cluster::weighted_lottery(&candidates, total, &seed).unwrap();
            *wins.get_mut(winner.0.as_str()).unwrap() += 1;
        }
        
        println!("Election Results (1000 rounds): {:?}", wins);
        
        // Verificação Aproximada (Lei dos Grandes Números)
        let sardine_runs = *wins.get("Sardine").unwrap();
        let whale_runs = *wins.get("Whale").unwrap();
        
        assert!(sardine_runs > 50 && sardine_runs < 150, "Sardine should have ~10% (50-150)");
        assert!(whale_runs > 500 && whale_runs < 700, "Whale should have ~60% (500-700)");
    }
}
