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
}
