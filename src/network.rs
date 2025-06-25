use std::sync::Arc;

use serde::{Serialize, Deserialize};

use crate::{NodeId, Proposal, Vote};

#[derive(Debug)]
pub enum NetworkError {
    SendError(String),
    ReceiveError(String),
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusMessage {
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
}

#[async_trait::async_trait]
pub trait NetworkAdapter: Send + Sync {
    async fn broadcast(&self, msg: ConsensusMessage) -> Result<(), NetworkError>;
    async fn send_to(&self, target: NodeId, msg: ConsensusMessage) -> Result<(), NetworkError>;
    fn set_message_handler(&mut self, handler: Arc<dyn Fn(ConsensusMessage) + Send + Sync>);
}