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
        let active_peers = peer_manager.get_active_peers();

        // Sugestão do usuário: não eleger um líder se não houver pares ativos.
        if active_peers.is_empty() {
            let mut leader_lock = self.current_leader.write().await;
            if leader_lock.is_some() {
                info!("Perdeu todos os pares, abdicando da liderança.");
                *leader_lock = None;
            }
            return;
        }

        let local_node_id = self.local_node.read().await.id.clone();
        let mut candidates = active_peers;
        candidates.insert(local_node_id.clone());

        // DEBUG: Imprime os candidatos em cada ciclo de eleição
        info!("[ELECTION DEBUG] Node {:?} candidates: {:?}", local_node_id, candidates);

        // Algoritmo de eleição simples: o nó com o maior ID vence.
        let new_leader = candidates.into_iter().max();

        let mut current_leader_lock = self.current_leader.write().await;
        
        if *current_leader_lock != new_leader {
            info!("👑 Novo líder eleito: {:?}", new_leader);
            *current_leader_lock = new_leader;
        }
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
        // Total prefix: 6 bytes [0, 36, 8, 1, 18, 32] -> hex 002408011220
        if bytes.len() == 38 && bytes.starts_with(&[0x00, 0x24, 0x08, 0x01, 0x12, 0x20]) {
             let pub_key_bytes = &bytes[6..];
             return Some(bs58::encode(pub_key_bytes).into_string());
        }

        tracing::warn!("NodeId {} does not match expected Ed25519 Identity pattern.", node_id_str);
        None
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
        let peer_id = libp2p::PeerId::from_str(node_id_str).unwrap();
        let bytes = peer_id.to_bytes();
        
        // Emulate the logic in node_id_to_address
        if bytes.len() == 38 && bytes.starts_with(&[0x00, 0x24, 0x08, 0x01, 0x12, 0x20]) {
             let pub_key_bytes = &bytes[6..];
             let addr = bs58::encode(pub_key_bytes).into_string();
             assert_eq!(addr, expected_addr);
        } else {
            panic!("Pattern match failed for known valid ID");
        }
    }
}
