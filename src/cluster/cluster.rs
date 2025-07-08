use std::{
    net::{SocketAddr, UdpSocket}, 
    sync::{Arc, RwLock}, 
    time::{SystemTime, UNIX_EPOCH}
};

use tokio::sync::oneshot;

use crate::{
    cluster_proto::{
        cluster_network_server::ClusterNetworkServer, HeartbeatMessage, ProposalMessage
    }, 
    env::{proposal::Proposal, AtlasEnv}, 
    network::adapter::{ClusterMessage, NetworkAdapter}, 
    peer_manager::{PeerCommand, PeerManager}, 
    utils::NodeId,
};
use super::{node::Node, service::ClusterService};

use crate::cluster_proto::Ack;

// TODO: Implement timeouts for heartbeats
// TODO: Implement retry logic for fail
// TODO: Implement periodic health checks
// TODO make new tests
// TODO: Implemente new metrics

/// Simulates a distributed cluster composed of multiple nodes.
///
/// This structure provides mechanisms for broadcasting messages,
/// simulating inter-node communication, and running cyclical simulations.
pub struct Cluster {
    /// The full set of nodes currently part of the cluster.
    pub local_env: AtlasEnv,
    pub network: Arc<RwLock<dyn NetworkAdapter>>,
    pub local_node: Node,
    pub peer_manager: Arc<RwLock<PeerManager>>,
    shutdown_sender: Option<oneshot::Sender<()>>,
}

impl Cluster {
    /// Initializes a new, empty cluster.
    pub fn new(
        env: AtlasEnv, 
        network: Arc<RwLock<dyn NetworkAdapter>>,
        node_id: NodeId,
    ) -> Self {
        let addr = network.read()
            .expect("Failed to acquire read lock")
            .get_address();
        
        Cluster {
            local_env: env.clone(),
            network,
            local_node: Self::set_local_node(node_id, &addr),
            peer_manager: Arc::clone(&env.peer_manager),
            shutdown_sender: None,
        }
    }

    /// Starts a gRPC server for this cluster node
    pub async fn serve_grpc(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let service = ClusterService::new(Arc::new(tokio::sync::RwLock::new(self.clone())));
        let addr = self.local_node.address.parse()?;

        let (tx, rx) = oneshot::channel();
        self.shutdown_sender = Some(tx); // salva o controle

        println!("🚀 Iniciando servidor gRPC em: {}", addr);

        // spawn o servidor em background
        tonic::transport::Server::builder()
            .add_service(ClusterNetworkServer::new(service))
            .serve_with_shutdown(addr, async {
                rx.await.ok();
            })
            .await?;


        Ok(())
    }

    pub fn shutdown_grpc(&mut self) {
        if let Some(tx) = self.shutdown_sender.take() {
            let _ = tx.send(()); // envia sinal para parar o servidor
            println!("🛑 Enviando sinal de shutdown para gRPC.");
        }
    }

    fn set_local_node(id: NodeId, addr: &str) -> Node {
        Node::new(id.into(), addr.to_string(), None, 0.0)
    }

    /// Adds a new node to the cluster by its unique identifier.
    pub fn add_node(&mut self, id: NodeId, stats: Node) -> Result<(), String> {
        let cmd = PeerCommand::Register(id, stats);
        let mut manager = self.peer_manager.write()
            .map_err(|_| "Failed to acquire write lock on peer manager")?;
        manager.handle_command(cmd);
        Ok(())
    }

    /// Broadcasts heartbeat messages from all nodes to all other peers.
    pub async fn broadcast_heartbeats(&self) -> Result<(), String> {
        let peers = {
            let manager = self.peer_manager.read()
                .map_err(|_| "Failed to acquire read lock on peer manager")?;
            manager.get_active_peers().iter().cloned().collect::<Vec<NodeId>>()
        };
        
        let sender_id = self.local_node.id.clone();
        let mut errors = Vec::new();

        for peer_id in peers {
            if peer_id != sender_id {
                if let Err(e) = self.send_heartbeat(peer_id.clone(), "broadcast".to_string()).await {
                    errors.push(format!("Failed to send heartbeat to {}: {}", peer_id, e));
                }
            }
        }

        if !errors.is_empty() {
            return Err(format!("Some heartbeats failed: {}", errors.join(", ")));
        }
        
        Ok(())
    }

    /// Sends a heartbeat message to a specific peer
    pub async fn send_heartbeat(&self, to: NodeId, msg: String) -> Result<(), String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "Failed to get system time")?
            .as_secs();
            
        println!("⏱️ Enviando heartbeat para [{}] em [{}] (cluster)", to, timestamp);

        // Get peer information with proper error handling
        let peer = {
            let manager = self.peer_manager.read()
                .map_err(|_| "Failed to acquire read lock on peer manager")?;
            manager.get_peer_stats(&to)
                .ok_or_else(|| format!("Peer {} not found", to.0))?
        };

        let heartbeat_msg = format!("{}: heartbeat from {}", peer.address, self.local_node.id);

        // Send heartbeat with error handling
        let network = self.network.write()
            .map_err(|_| "Failed to acquire write lock on network adapter")?;
            
        network.send_heartbeat(
            self.local_node.id.clone(), 
            peer.clone(), 
            heartbeat_msg.clone()
        ).await
        .map_err(|e| format!("Network error: {:?}", e))?;

        println!("✅ Heartbeat enviado com sucesso para {}", to);
        Ok(())
    }

    /// Sends a heartbeat message to a specific peer
    pub async fn submit_proposal(&self, proposal: Proposal) -> Result<Ack, String> {
        println!("🚀 Submetendo proposta local: {:?}", proposal);

        // 1. Processa localmente (pode ser handle_proposal ou lógica própria)
        let ack_local = self.handle_proposal(proposal.clone().into_proto()).await;

        // 2. Propaga para outros peers
        let peers = {
            let manager = self.peer_manager.read()
                .map_err(|_| "Failed to acquire read lock on peer manager")?;
            manager.get_active_peers().iter().cloned().collect::<Vec<NodeId>>()
        };

        let network = self.network.write()
            .map_err(|_| "Failed to acquire write lock on network adapter")?;

        let mut errors = Vec::new();

        for peer_id in peers {
            if peer_id != self.local_node.id {
                // Aqui você pode usar send_to ou broadcast, dependendo da sua arquitetura
                let msg = ClusterMessage::Proposal {
                    proposal: proposal.clone(),
                    public_key: vec![],
                    signature: vec![],
                };
                if let Err(e) = network.send_to(peer_id.clone(), msg).await {
                    errors.push(format!("Erro ao enviar para {}: {:?}", peer_id, e));
                }
            }
        }

        if !errors.is_empty() {
            println!("⚠️ Alguns envios falharam: {:?}", errors);
        } else {
            println!("✅ Proposta propagada para todos os peers");
        }

        Ok(ack_local)
    }

    /// Handles incoming heartbeat messages
    pub async fn handle_heartbeat(&self, msg: HeartbeatMessage) -> Ack {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        println!("⏱️ Heartbeat recebido de [{}] em [{}]", msg.from, msg.timestamp);
        
        // Update peer's last seen timestamp
        if let Ok(mut manager) = self.peer_manager.write() {
            // Note: This would need a method to update last_seen in PeerManager
            // manager.update_peer_last_seen(&msg.from, timestamp);
        }
        
        Ack {
            received: true,
            message: format!("ACK recebido por {} em {}", self.local_node.id, timestamp),
        }
    }

    pub async fn handle_proposal(&self, msg: ProposalMessage) -> Ack {
        let proposal = Proposal::from_proto(msg);

        // Aqui você pode adicionar lógica de validação, armazenamento, broadcast, etc.
        println!("📨 Proposta recebida: {:?}", proposal);

        Ack {
            received: true,
            message: format!("Proposta {} recebida por {}", proposal.id, self.local_node.id),
        }
    }

    /// Gets the number of active peers in the cluster
    pub fn get_peer_count(&self) -> Result<usize, String> {
        let manager = self.peer_manager.read()
            .map_err(|_| "Failed to acquire read lock on peer manager")?;
        Ok(manager.get_active_peers().len())
    }

    /// Checks if a specific peer is active
    pub fn is_peer_active(&self, peer_id: &NodeId) -> Result<bool, String> {
        let manager = self.peer_manager.read()
            .map_err(|_| "Failed to acquire read lock on peer manager")?;
        Ok(manager.get_peer_stats(peer_id).is_some())
    }

    pub fn clone(&self) -> Self {
        Cluster {
            local_env: self.local_env.clone(),
            network: Arc::clone(&self.network),
            local_node: self.local_node.clone(),
            peer_manager: Arc::clone(&self.peer_manager),
            shutdown_sender: None, // não clonamos o sender!
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
