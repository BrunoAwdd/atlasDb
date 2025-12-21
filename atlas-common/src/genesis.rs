use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the initial state of the ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisState {
    /// Initial token allocations: Address (Base58) -> Amount (u64)
    pub allocations: HashMap<String, u64>,
}

impl GenesisState {
    pub fn new() -> Self {
        Self {
            allocations: HashMap::new(),
        }
    }
}
