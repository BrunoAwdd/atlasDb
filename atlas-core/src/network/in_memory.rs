use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use tokio::sync::mpsc::{self, Sender, Receiver};
use atlas_common::utils::NodeId;
use crate::network::message::{ClusterMessage, NetworkError};
use crate::network::traits::Network;

#[derive(Clone)]
pub struct InMemoryNetwork {
    pub id: NodeId,
    peers: Arc<Mutex<HashMap<NodeId, Sender<ClusterMessage>>>>,
    message_handler: Arc<Mutex<Option<Box<dyn Fn(ClusterMessage) + Send + Sync>>>>,
}

impl InMemoryNetwork {
    pub fn new(id: NodeId) -> (Self, Sender<ClusterMessage>, Receiver<ClusterMessage>) {
        let (tx, rx) = mpsc::channel(100);
        
        (Self {
            id,
            peers: Arc::new(Mutex::new(HashMap::new())),
            message_handler: Arc::new(Mutex::new(None)),
        }, tx, rx)
    }

    pub fn add_peer(&self, peer_id: NodeId, sender: Sender<ClusterMessage>) {
        self.peers.lock().unwrap().insert(peer_id, sender);
    }
    
    pub async fn run(&self, mut rx: Receiver<ClusterMessage>) {
        while let Some(msg) = rx.recv().await {
            let handler = self.message_handler.lock().unwrap();
            if let Some(h) = handler.as_ref() {
                h(msg);
            }
        }
    }
}

#[async_trait]
impl Network for InMemoryNetwork {
    async fn send_to(&self, peer: NodeId, message: ClusterMessage) -> Result<(), NetworkError> {
        let sender = {
            let peers = self.peers.lock().unwrap();
            peers.get(&peer).cloned()
        };

        if let Some(sender) = sender {
            sender.send(message).await.map_err(|_| NetworkError::SendError(peer.0))?;
            Ok(())
        } else {
            Err(NetworkError::PeerNotFound(peer.0))
        }
    }

    async fn broadcast(&self, message: ClusterMessage) -> Result<(), NetworkError> {
        let peers = {
            let peers = self.peers.lock().unwrap();
            peers.clone()
        };

        for (_peer_id, sender) in peers {
            let _ = sender.send(message.clone()).await;
        }
        Ok(())
    }

    async fn connected_peers(&self) -> Vec<NodeId> {
        self.peers.lock().unwrap().keys().cloned().collect()
    }

    fn set_message_handler(&self, handler: Box<dyn Fn(ClusterMessage) + Send + Sync>) {
        let mut h = self.message_handler.lock().unwrap();
        *h = Some(handler);
    }
}
