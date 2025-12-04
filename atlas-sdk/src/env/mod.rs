pub mod consensus;
pub mod node;
pub mod proposal;
pub mod vote_data;

use consensus::types::ConsensusResult;

pub trait Callback: Fn(ConsensusResult) + Send + Sync {}
impl<T> Callback for T where T: Fn(ConsensusResult) + Send + Sync {}