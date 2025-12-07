use serde::{Serialize, Deserialize};

use atlas_sdk::env::proposal::Proposal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxRequest {
    GetTxs { txids: Vec<[u8;32]> },
    GetState { height: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxBundle {
    Txs { txs: Vec<Vec<u8>> },
    State { proposals: Vec<Proposal> },
}