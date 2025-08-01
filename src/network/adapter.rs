use std::sync::Arc;
use std::fmt::Debug;

use serde::{Serialize, Deserialize};

use crate::{
    cluster_proto::VoteMessage, 
    env::{
        consensus::Vote,
        proposal::Proposal
    }, 
    utils::NodeId, Node
};
use super::error::NetworkError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteData {
    pub proposal_id: String,
    pub vote: Vote,
    pub voter: NodeId,
}

impl VoteData {
    pub fn into_proto(self, public_key: Vec<u8>, signature: Vec<u8>) -> VoteMessage {
        VoteMessage {
            proposal_id: self.proposal_id,
            voter_id: self.voter.0,
            vote: self.vote as i32,
            signature,
            public_key,
        }
    }

    pub fn from_proto(msg: VoteMessage) -> Self {
        let vote = Vote::try_from(msg.vote).unwrap_or(Vote::Abstain);
        Self {
            proposal_id: msg.proposal_id,
            vote: vote,
            voter: NodeId(msg.voter_id),
        }
    }

    pub fn into_cluster_message(self, public_key: Vec<u8>, signature: Vec<u8>) -> ClusterMessage {
        ClusterMessage::Vote {
            proposal_id: self.proposal_id,
            vote: self.vote,
            voter: self.voter,
            public_key: public_key,
            signature: signature,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterMessage {
    Proposal {
        proposal: Proposal,
    },
    Vote {
        proposal_id: String,
        vote: Vote,
        voter: NodeId,
        public_key: Vec<u8>,
        signature: Vec<u8>,
    },
    VoteBatch {
        votes: Vec<VoteData>,
        public_key: Vec<u8>,
        signature: Vec<u8>,

    },
    Heartbeat {
        sender: NodeId,
        receiver: NodeId,
        from: NodeId,
        timestamp: u64,
    },
}

#[async_trait::async_trait]
pub trait NetworkAdapter: Send + Sync + Debug {
    fn get_address(&self) -> String;
    async fn broadcast(&self, msg: ClusterMessage) -> Result<(), NetworkError>;
    async fn send_to(&self, target: Node, msg: ClusterMessage) -> Result<ClusterMessage, NetworkError>;
    async fn send_votes(&self, target: Node, votes: ClusterMessage) -> Result<(), NetworkError>;
    async fn send_proposal(&self, target: Node, proposals: Proposal) -> Result<(), NetworkError>;
    fn set_message_handler(&mut self, handler: Arc<dyn Fn(ClusterMessage) + Send + Sync>);
    async fn send_heartbeat(&self, sender: NodeId, receiver: Node) -> Result<ClusterMessage, NetworkError>;
}