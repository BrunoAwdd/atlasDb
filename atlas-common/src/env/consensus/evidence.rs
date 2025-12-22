
use serde::{Serialize, Deserialize};
use crate::env::vote_data::VoteData;

/// Represents the cryptographic evidence that a validator voted twice 
/// for the same view/phase but with different values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquivocationEvidence {
    pub vote_a: VoteData,
    pub vote_b: VoteData,
}

impl EquivocationEvidence {
    pub fn offender(&self) -> &crate::utils::NodeId {
        &self.vote_a.voter
    }
}
