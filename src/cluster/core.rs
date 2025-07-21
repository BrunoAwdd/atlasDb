use std::{
    sync::{Arc, RwLock}, 
};

use tokio::sync::oneshot;

use crate::{
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
    pub local_env: Arc<RwLock<AtlasEnv>>,
    pub network: Arc<RwLock<dyn NetworkAdapter>>,
    pub local_node: Node,
    pub peer_manager: Arc<RwLock<PeerManager>>,
    pub shutdown_sender: Option<oneshot::Sender<()>>,
}

impl Cluster {
    /// Initializes a new, empty cluster.
    pub fn new(
        env: Arc<RwLock<AtlasEnv>>, 
        network: Arc<RwLock<dyn NetworkAdapter>>,
        node_id: NodeId,
    ) -> Self {
        let addr = network.read()
            .expect("Failed to acquire read lock")
            .get_address();

        let peer_manager = Arc::clone(&env.read().expect("Failed to acquire read lock").peer_manager);
        
        Cluster {
            local_env: env,
            network,
            local_node: Self::set_local_node(node_id, &addr),
            peer_manager,
            shutdown_sender: None,
        }
    }

    fn set_local_node(id: NodeId, addr: &str) -> Node {
        Node::new(id.into(), addr.to_string(), None, 0.0)
    }

}
