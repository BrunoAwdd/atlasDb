
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

    #[serde(with = "hex::serde")]
    pub signature: [u8; 64],
    pub public_key: Vec<u8>,
}
    
impl Proposal {
    pub fn from_proto(msg: ProposalMessage) -> Result<Self, String> {
        let signature: [u8; 64] = msg.signature.try_into()
            .map_err(|_| "Assinatura inválida: deve ter 64 bytes".to_string())?;

        Ok(Proposal {
            id: msg.id,
            proposer: NodeId(msg.proposer_id),
            content: msg.content,
            parent: if msg.parent_id.is_empty() { None } else { Some(msg.parent_id) },
            signature,
            public_key: msg.public_key.to_vec(),
        })
    }

    pub fn into_proto(&self) -> ProposalMessage {
        ProposalMessage {
            id: self.id.clone(),
            proposer_id: self.proposer.0.clone(),
            content: self.content.clone(),
            parent_id: self.parent.clone().unwrap_or_default(),
            signature: self.signature.clone().into(),
            public_key: self.public_key.clone().into(),
        }
    }

    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}