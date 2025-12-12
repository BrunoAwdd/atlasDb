use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub type EntryId = String;
pub type Address = String;
pub type AssetId = String;
pub type Hash = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LegKind {
    Debit,
    Credit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Leg {
    pub account: Address,
    pub asset: AssetId,
    pub kind: LegKind,
    pub amount: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub entry_id: EntryId,
    pub legs: Vec<Leg>,
    pub tx_hash: Hash,
    pub memo: Option<String>,
    pub block_height: u64,
    pub timestamp: i64,
    // Optional: map of previous entry IDs for each account involved, for auditing chains
    pub prev_for_account: HashMap<Address, EntryId>,
}

impl LedgerEntry {
    pub fn new(
        entry_id: EntryId,
        legs: Vec<Leg>,
        tx_hash: Hash,
        block_height: u64,
        timestamp: i64,
        memo: Option<String>,
    ) -> Self {
        Self {
            entry_id,
            legs,
            tx_hash,
            memo,
            block_height,
            timestamp,
            prev_for_account: HashMap::new(),
        }
    }
}
