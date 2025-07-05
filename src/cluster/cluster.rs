use std::{
    net::{SocketAddr, UdpSocket}, 
    sync::{Arc, RwLock}, time::{SystemTime, UNIX_EPOCH}
};

use crate::{
    cluster_proto::{
        HeartbeatMessage, 
        cluster_network_server::ClusterNetworkServer
    }, 
    env::AtlasEnv, 
    network::adapter::NetworkAdapter, 
    peer_manager::{PeerCommand, PeerManager}, 
    utils::NodeId
};
use super::{node::Node, service::ClusterService};

use crate::cluster_proto::Ack;



/// Simulates a distributed cluster composed of multiple nodes.
///
/// This structure provides mechanisms for broadcasting messages,
/// simulating inter-node communication, and running cyclical simulations.
#[derive(Clone)]
pub struct Cluster {
    /// The full set of nodes currently part of the cluster.
    pub local_env: AtlasEnv,
    pub network: Arc<RwLock<dyn NetworkAdapter>>,
    pub local_node: Node,
    pub peer_manager: Arc<RwLock<PeerManager>>,
}

impl Cluster {
    /// Initializes a new, empty cluster.
    pub fn new(
        env: AtlasEnv, 
        network: Arc<RwLock<dyn NetworkAdapter>>,
        node_id: NodeId,
    ) -> Self {
        let addr = network.read().expect("Failed to acquire read lock").get_address();
        Cluster {
            local_env: env.clone(),
            network,
            local_node: Self::set_local_node(node_id, &addr),
            peer_manager: Arc::clone(&env.peer_manager),
        }
    }

    pub async fn serve_grpc(self, addr: SocketAddr) {
        let service = ClusterService::new(Arc::new(tokio::sync::RwLock::new(self.clone())));

        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(ClusterNetworkServer::new(service))
                .serve(addr)
                .await
                .unwrap();
        });

        println!("🚀 Servidor gRPC em: {}", addr);
    }

    fn set_local_node(id: NodeId, addr: &str) -> Node {
        Node::new(id.into(), addr.to_string(), None, 0.0)
    }

    /// Adds a new node to the cluster by its unique identifier.
    pub fn add_node(&mut self, id: NodeId, stats: Node) {
        let cmd = PeerCommand::Register(id, stats);
        let mut manager = self.peer_manager.write().expect("Failed to acquire write lock");
        manager.handle_command(cmd);
    }

    /// Broadcasts heartbeat messages from all nodes to all other peers.
    pub fn broadcast_heartbeats(&self) {
        let peers: Vec<NodeId> = self.peer_manager.read().expect("Failed to acquire read lock").get_active_peers().iter().cloned().collect();
        let sender_id = self.local_node.id.clone();

        for peer_id in peers {
            if peer_id != sender_id {
                self.send_heartbeat(peer_id, "any".to_string());
            }
        }
    }

    pub async fn send_heartbeat(&self, to: NodeId, msg: String)  {
        println!("⏱️ Enviando heartbeat para [{}] em [{}] (cluster)", to, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());

        let peer = self.peer_manager.read().expect("Failed to acquire read lock")
                    .get_peer_stats(&to)
                    .expect("Peer not found");
        let msg = format!("{}: heartbeat from {}", peer.address, self.local_node.id);

        let _ = self.network
            .write()
            .expect("Failed to acquire write lock")
            .send_heartbeat(
                self.local_node.id.clone(), 
                peer.clone(), 
                msg.clone()
            ).await;
    }

    pub async fn handle_heartbeat(&self, msg: HeartbeatMessage) -> Ack {
        println!("⏱️ Heartbeat recebido de [{}] em [{}]", msg.from, msg.timestamp);
        Ack {
            received: true,
            message: format!("ACK recebido por {}", self.local_node.id),
        }
    }




}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, RwLock};
    use crate::{
        cluster::node::Node, env::AtlasEnv, network::{
            adapter::{ClusterMessage, NetworkAdapter},
            error::NetworkError
        }, peer_manager::{PeerCommand, PeerManager}, utils::NodeId
    };
    use super::Cluster;
    
    use async_trait::async_trait;
    /*
    #[derive(Default, Clone)]
    pub struct MockNetworkAdapter {
        pub messages_sent: Arc<Mutex<Vec<(Option<NodeId>, ClusterMessage)>>>,
        pub heartbeats_sent: Arc<Mutex<Vec<(NodeId, NodeId, String)>>>,
        pub handler: Arc<Mutex<Option<Arc<dyn Fn(ClusterMessage) + Send + Sync>>>>,
    }

    impl MockNetworkAdapter {
        fn new() -> Self {
            MockNetworkAdapter::default()
        }
    }

    #[async_trait]
    impl NetworkAdapter for MockNetworkAdapter {
        async fn broadcast(&self, msg: ClusterMessage) -> Result<(), NetworkError> {
            self.messages_sent.lock().unwrap().push((None, msg.clone()));
            if let Some(h) = self.handler.lock().unwrap().clone() {
                h(msg);
            }
            Ok(())
        }
    
        async fn send_to(&self, target: NodeId, msg: ClusterMessage) -> Result<(), NetworkError> {
            self.messages_sent.lock().unwrap().push((Some(target.clone()), msg.clone()));
            if let Some(h) = self.handler.lock().unwrap().clone() {
                h(msg);
            }
            Ok(())
        }
    
        fn set_message_handler(&mut self, handler: Arc<dyn Fn(ClusterMessage) + Send + Sync>) {
            *self.handler.lock().unwrap() = Some(handler);
        }
    
        fn send_heartbeat(&self, sender: NodeId, receiver: NodeId, msg: String) -> ClusterMessage {
            self.heartbeats_sent.lock().unwrap().push((sender.clone(), receiver.clone(), msg.clone()));
            ClusterMessage::Vote {
                proposal_id: "heartbeat".to_string(),
                vote: crate::env::consensus::Vote::Yes, // placeholder
                voter: sender,
                public_key: vec![],
                signature: vec![],
            }
        }
    }
    
    #[test]
    fn test_cluster_initialization_with_nodes() {
        let mock = Arc::new(RwLock::new(MockNetworkAdapter::new()));
        let network = mock as Arc<RwLock<dyn NetworkAdapter>>;
        let peer_manager = Arc::new(RwLock::new(PeerManager::new(2, 2)));

        let env = AtlasEnv::new(
            Arc::clone(&network),
            Arc::new(|_| {}),
            Arc::clone(&peer_manager)
        );
    
        let mut cluster = Cluster::new(env, network, peer_manager, NodeId("node-X".into()));
        cluster.add_node(NodeId("node-A".into()), Node {
            reliability_score: 1.0,
            latency: Some(10),
            last_seen: 0,
            address: "127.0.0.1".to_string(),
            id: NodeId("node-A".into()),
        });
        cluster.add_node(NodeId("node-B".into()), Node {
            reliability_score: 0.9,
            latency: Some(20),
            last_seen: 0,
            address: "127.0.0.1".to_string(),
            id: NodeId("node-B".into()),
        });

        let nodes: Vec<NodeId> = cluster.local_env.peer_manager.read().unwrap().get_active_peers().iter().cloned().collect();
    
        assert_eq!(nodes.len(), 2);
        assert!(nodes.contains(&NodeId("node-A".into())));
    }
    
    #[test]
    fn test_heartbeat_broadcast_sends_messages() {
        let mock = Arc::new(RwLock::new(MockNetworkAdapter::new()));
        let network = mock.clone() as Arc<RwLock<dyn NetworkAdapter>>;
        let peer_manager = Arc::new(RwLock::new(PeerManager::new(2, 2)));
        let env = AtlasEnv::new(
            Arc::clone(&network),
            Arc::new(|_| {}),
            Arc::clone(&peer_manager)
        );
    
        let mut cluster = Cluster::new(env, network.clone(), peer_manager, NodeId("node-X".into()));
        cluster.add_node(NodeId("peer-1".into()), Node {
            reliability_score: 1.0,
            latency: Some(10),
            last_seen: 0,
            address: "127.0.0.1".to_string(),
            id: NodeId("peer-1".into()),
        });
        cluster.add_node(NodeId("peer-2".into()), Node {
            reliability_score: 0.9,
            latency: Some(20),
            last_seen: 0,
            address: "127.0.0.1".to_string(),
            id: NodeId("peer-2".into()),
        });
    
        cluster.broadcast_heartbeats();

        let guard = mock.read().expect("Failed to acquire read lock");
        let sent = guard.heartbeats_sent.lock().unwrap();

        assert!(!sent.is_empty());
        assert!(sent.iter().any(|(_, to, msg)| to.0 == "peer-1" && msg.contains("heartbeat")));
    }
    
    #[test]
    fn test_peer_manager_tracks_peers() {
        let mock = Arc::new(RwLock::new(MockNetworkAdapter::new()));
        let network = mock as Arc<RwLock<dyn NetworkAdapter>>;
        let peer_manager = Arc::new(RwLock::new(PeerManager::new(2, 2)));
        let env = AtlasEnv::new(
            Arc::clone(&network),
            Arc::new(|_| {}),
            Arc::clone(&peer_manager)
        );

        let cluster = Cluster::new(env, network.clone(), peer_manager, NodeId("node-X".into()));

        for i in 0..5 {
            let id = NodeId(format!("peer-{}", i));
            let stats = Node {
                reliability_score: 1.0,
                latency: Some(10),
                last_seen: 0,
                address: "127.0.0.1".to_string(),
                id: id.clone(),
            };
            cluster.peer_manager.write().expect("Failed to acquire write lock").handle_command(PeerCommand::Register(id.clone(), stats));
        }

        assert_eq!(cluster.peer_manager.read().expect("Failed to acquire read lock").get_active_peers().len(), 5);
    }

    */
}
