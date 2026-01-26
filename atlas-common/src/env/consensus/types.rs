use std::fmt;

use serde::{Serialize, Deserialize};

/// Represents a binary vote from a node regarding a proposal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Vote {
    Yes,
    No,
    Abstain
}

/// Phases of the BFT consensus protocol.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConsensusPhase {
    /// Initial phase: Nodes vote on the proposal validity.
    Prepare,
    /// Intermediate phase: Nodes confirm they have seen a quorum of Prepares.
    PreCommit,
    /// Final phase: Nodes vote to commit the proposal.
    Commit,
}

impl Default for ConsensusPhase {
    fn default() -> Self {
        Self::Prepare
    }
}

impl From<Vote> for i32 {
    fn from(v: Vote) -> Self {
        match v {
            Vote::Yes => 0,
            Vote::No => 1,
            Vote::Abstain => 2,
        }
    }
}

impl std::convert::TryFrom<i32> for Vote {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Vote::Yes),
            1 => Ok(Vote::No),
            2 => Ok(Vote::Abstain),
            _ => Err(()),
        }
    }
}
impl fmt::Display for Vote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Vote::Yes => "Yes",
            Vote::No => "No",
            Vote::Abstain => "Abstain",
        };
        write!(f, "{}", s)
    }
}

/// The result of consensus evaluation for a single proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    /// Whether the proposal reached quorum and was approved.
    pub approved: bool,

    /// The number of affirmative (Yes) votes received.
    pub votes_received: usize,

    /// The proposal ID this result corresponds to.
    pub proposal_id: String,

    /// The phase for which the consensus result is valid.
    #[serde(default)]
    pub phase: ConsensusPhase,
}