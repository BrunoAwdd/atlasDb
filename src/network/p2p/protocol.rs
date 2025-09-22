use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRequest {
    pub txids: Vec<[u8;32]>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxBundle {
    pub txs: Vec<Vec<u8>>,
}