use std::{net::SocketAddr, sync::Arc};

use tokio::sync::{oneshot, Mutex, RwLock};

use crate::{
    auth::Authenticator, 
    env::AtlasEnv, 
    network::adapter::NetworkAdapter, 
    peer_manager::PeerManager,
    utils::NodeId
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
    pub network: Arc<dyn NetworkAdapter>,
    pub local_node: Node,
    pub peer_manager: Arc<RwLock<PeerManager>>,
    pub shutdown_sender: Mutex<Option<oneshot::Sender<()>>>,
    pub auth: Arc<RwLock<dyn Authenticator>>,
}

impl Cluster {
    /// Initializes a new, empty cluster.
    pub fn new(
        env: AtlasEnv, 
        network: Arc<dyn NetworkAdapter>,
        node_id: NodeId,
        auth: Arc<RwLock<dyn Authenticator>>,
    ) -> Self {
        let addr = network.get_address();

        let peer_manager = Arc::clone(&env.peer_manager);
        
        Cluster {
            local_env: env,
            network,
            local_node: Self::set_local_node(node_id, &addr),
            peer_manager,
            shutdown_sender: Mutex::new(None),
            auth
        }
    }

    fn set_local_node(id: NodeId, addr: &str) -> Node {
        Node::new(id.into(), addr.to_string(), None, 0.0)
    }

}
