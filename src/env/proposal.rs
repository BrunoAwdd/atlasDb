
use serde::{Serialize, Deserialize};
use crate::{cluster_proto::ProposalMessage, utils::NodeId};

/// A proposal to mutate or modify shared graph state.
///
/// Each proposal is authored by a node and uniquely identified.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// Unique identifier for the proposal.
    pub id: String,

    /// The node that submitted the proposal.
    pub proposer: NodeId,

    /// The proposed content or payload (e.g., graph update).
    pub content: String,

    pub parent: Option<String>, // Optional parent proposal ID for versioning
}

impl Proposal {
    pub fn from_proto(msg: ProposalMessage) -> Self {
        Proposal {
            id: msg.id,
            proposer: NodeId(msg.proposer_id),
            content: msg.content,
            parent: if msg.parent_id.is_empty() { None } else { Some(msg.parent_id) },
        }
    }

    pub fn into_proto(&self) -> ProposalMessage {
        ProposalMessage {
            id: self.id.clone(),
            proposer_id: self.proposer.0.clone(),
            content: self.content.clone(),
            parent_id: self.parent.clone().unwrap_or_default(),
            signature: vec![],
            public_key: vec![],
        }
    }
}