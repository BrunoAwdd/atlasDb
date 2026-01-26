use std::fmt::Debug;

use serde::{Serialize, Deserialize};

use crate::{
    env::{
        consensus::types::{Vote, ConsensusPhase}
    },
    utils::NodeId,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteData {
    pub proposal_id: String,
    pub vote: Vote,
    pub voter: NodeId,
    #[serde(default)]
    pub phase: ConsensusPhase,
    #[serde(default)]
    pub view: u64,
    #[serde(default)]
    pub height: u64, // New field for Height Awareness
    #[serde(with = "hex::serde")]
    pub signature: [u8; 64],
    pub public_key: Vec<u8>,
}
impl VoteData {
    pub fn from_json(json: &str) -> serde_json::Result<Self> {
        serde_json::from_str(json)
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    pub fn bytes(&self) -> Vec<u8> {
        bincode::serialize(self).expect("serialize vote")
    }
}

#[derive(Serialize)]
struct VoteSignView<'a> {
    id:       &'a str,
    vote:     &'a Vote,
    voter:    &'a NodeId,
    phase:    &'a ConsensusPhase,
    view:     u64,
    height:   u64, // Include in signature
}

pub fn vote_signing_bytes(v: &VoteData) -> Vec<u8> {
    // bincode (rápido) ou serde_json (debugável). Use sempre o mesmo!
    bincode::serialize(&VoteSignView {
        id: &v.proposal_id,
        vote: &v.vote,
        voter: &v.voter,
        phase: &v.phase,
        view: v.view,
        height: v.height,
    }).expect("serialize sign view")
}