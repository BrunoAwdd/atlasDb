
use serde::{Serialize, Deserialize};
use crate::utils::NodeId;

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
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}