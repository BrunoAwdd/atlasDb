
use serde::{Serialize, Deserialize};
use crate::env::consensus::types::Vote;
use crate::utils::NodeId;

/// Represents the cryptographic evidence that a validator voted twice 
/// for the same view/phase but with different values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivocationEvidence {
    pub offender: NodeId,
    pub view: u64,
    pub phase_step: String, // "Prepare" or "Commit"
    pub vote_a: Vote,
    pub vote_b: Vote,
    pub proposal_a: String,
    pub proposal_b: String,
}
