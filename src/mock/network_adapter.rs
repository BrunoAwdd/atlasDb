use std::{collections::HashMap, fmt, sync::{Arc, Mutex}};

use async_trait::async_trait;

use crate::cluster::node::Node;

use super::super::network::adapter::NetworkAdapter;
use super::super::utils::NodeId;
use super::super::network::{error::NetworkError, adapter::ClusterMessage};

#[derive(Default, Clone)]
pub struct MockNetworkAdapter {
    pub node_id: NodeId,
    pub bus: Arc<MockNetworkBus>,
    pub messages_sent: Arc<Mutex<Vec<(Option<NodeId>, ClusterMessage)>>>,
    pub heartbeats_sent: Arc<Mutex<Vec<(NodeId, NodeId, String)>>>,
    pub handler: Arc<Mutex<Option<Arc<dyn Fn(ClusterMessage) + Send + Sync>>>>,
    pub messages_received: Arc<Mutex<Vec<ClusterMessage>>>,
}

impl MockNetworkAdapter {
    pub fn new() -> Self {
        MockNetworkAdapter::default()
    }
}

impl fmt::Debug for MockNetworkAdapter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MyAdapter")
         .field("handler", &"Fn handler (not Debug)")
         .finish()
    }
}

#[async_trait]
impl NetworkAdapter for MockNetworkAdapter {
    fn get_address(&self) -> String {
        "Mock".to_string()
    }

    async fn broadcast(&self, msg: ClusterMessage) -> Result<(), NetworkError> {
        self.messages_sent.lock().unwrap().push((None, msg.clone()));
        if let Some(h) = self.handler.lock().unwrap().clone() {
            h(msg);
        }
        Ok(())
    }

    async fn send_to(&self, target: NodeId, msg: ClusterMessage) -> Result<(), NetworkError> {
        self.bus.send(&target, msg);
        Ok(())
    }

    fn set_message_handler(&mut self, handler: HandlerFn) {
        println!("Esta aqui [{}]", self.node_id);
        self.bus.register_node(self.node_id.clone(), handler);
    }

    async fn send_heartbeat(&self, sender: NodeId, receiver: Node, msg: String) -> Result<(ClusterMessage), NetworkError> {
        self.heartbeats_sent.lock().unwrap().push((sender.clone(), receiver.id.clone(), msg.clone()));
        let message = ClusterMessage::Vote {
            proposal_id: "heartbeat".to_string(),
            vote: crate::env::consensus::Vote::Yes, // placeholder
            voter: sender,
            public_key: vec![],
            signature: vec![],
        };

        println!("Sending heartbeat: {:?}", message);

        Ok(message)
    }
}

impl fmt::Debug for MockNetworkBus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MyAdapter")
         .field("handler", &"Fn handler (not Debug)")
         .finish()
    }
}


pub type HandlerFn = Arc<dyn Fn(ClusterMessage) + Send + Sync>;

#[derive(Default, Clone)]
pub struct MockNetworkBus {
    pub nodes: Arc<Mutex<HashMap<NodeId, HandlerFn>>>,
}

impl MockNetworkBus {
    pub fn register_node(&self, id: NodeId, handler: HandlerFn) {
        self.nodes.lock().unwrap().insert(id, handler);
    }

    pub fn send(&self, target: &NodeId, msg: ClusterMessage) {
        println!("Sending message to (Bus) [{}]", target);
        println!("[Bus] recebeu mensagem:\n{:#?}", msg);
        if let Some(handler) = self.nodes.lock().unwrap().get(target) {
            handler(msg);
        }
    }

    pub fn broadcast(&self, from: &NodeId, msg: ClusterMessage) {
        for (id, handler) in self.nodes.lock().unwrap().iter() {
            if id != from {
                handler(msg.clone());
            }
        }
    }
}