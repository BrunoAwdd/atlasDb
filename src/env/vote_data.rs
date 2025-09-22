use std::fmt::Debug;

use serde::{Serialize, Deserialize};

use crate::{
    env::{
        consensus::Vote,
    }, 
    utils::NodeId
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteData {
    pub proposal_id: String,
    pub vote: Vote,
    pub voter: NodeId,
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
}

pub fn vote_signing_bytes(v: &VoteData) -> Vec<u8> {
    // bincode (rápido) ou serde_json (debugável). Use sempre o mesmo!
    bincode::serialize(&VoteSignView {
        id: &v.proposal_id,
        vote: &v.vote,
        voter: &v.voter,
    }).expect("serialize sign view")
}