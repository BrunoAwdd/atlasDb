use std::{net::SocketAddr, sync::Arc};


use tokio::sync::{oneshot, Mutex, RwLock};
use atlas_common::{
    auth::Authenticator,
    utils::NodeId
};

use crate::{
    config::Config, 
    env::runtime::AtlasEnv,
    peer_manager::PeerManager, 
    Graph, 

};
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


    // @TODO: Is here the best place to save the state?
    // Returns a Config object representing the current state, which can be saved to disk by the caller.
    pub async fn export_config(&self) -> Config {
        let local_node = self.local_node.read().await;
        // Parsing is safe here as node address is validated on creation
        let socket: SocketAddr = local_node.address.parse().unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], 0)));
        
        Config {
            node_id: local_node.id.clone(),
            address:  socket.ip().to_string(),
            port: socket.port(),
            quorum_policy: self.local_env.engine.lock().await.evaluator.policy.clone(),
            graph: Graph::new(), // TODO: Graph state?
            storage: self.local_env.storage.read().await.clone(),
            peer_manager: self.peer_manager.read().await.clone(),
            data_dir: self.local_env.data_dir.clone(),
        }
    }

    pub async fn elect_leader(&self) {
        let local_id = self.local_node.read().await.id.clone();
        crate::cluster::election::elect_leader(
            local_id,
            &self.peer_manager,
            &self.local_env.storage,
            &self.current_leader
        ).await;
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
