use std::sync::Arc;
use std::fmt::Debug;

use serde::{Serialize, Deserialize};

use crate::{
    env::consensus::{Proposal, Vote}, utils::NodeId, Node
};
use super::error::NetworkError;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterMessage {
    Proposal {
        proposal: Proposal,
        public_key: Vec<u8>,
        signature: Vec<u8>,
    },
    Vote {
        proposal_id: String,
        vote: Vote,
        voter: NodeId,
        public_key: Vec<u8>,
        signature: Vec<u8>,
    },
    Heartbeat {
        sender: NodeId,
        receiver: NodeId,
        msg: String,
    },
}

#[async_trait::async_trait]
pub trait NetworkAdapter: Send + Sync + Debug {
    fn get_address(&self) -> String;
    async fn broadcast(&self, msg: ClusterMessage) -> Result<(), NetworkError>;
    async fn send_to(&self, target: NodeId, msg: ClusterMessage) -> Result<(), NetworkError>;
    fn set_message_handler(&mut self, handler: Arc<dyn Fn(ClusterMessage) + Send + Sync>);
    async fn send_heartbeat(&self, sender: NodeId, receiver: Node, msg: String) -> Result<(ClusterMessage), NetworkError>;
}