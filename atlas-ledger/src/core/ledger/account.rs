use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use atlas_common::entry::{AssetId, EntryId};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccountState {
    pub balances: HashMap<AssetId, u128>,
    pub last_entry_id: Option<EntryId>,
    pub nonce: u64,
}

impl AccountState {
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            last_entry_id: None,
            nonce: 0,
        }
    }

    pub fn get_balance(&self, asset: &AssetId) -> u128 {
        *self.balances.get(asset).unwrap_or(&0)
    }
}
