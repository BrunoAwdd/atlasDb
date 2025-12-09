use serde::{Deserialize, Serialize};
use thiserror::Error;
use atlas_common::env::proposal::Proposal;
use atlas_common::env::vote_data::VoteData;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClusterMessage {
    Proposal(Proposal),
    Vote(VoteData),
}

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Failed to send message to peer {0}")]
    SendError(String),
    #[error("Peer {0} not found")]
    PeerNotFound(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
