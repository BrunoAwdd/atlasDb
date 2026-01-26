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

// Hardcoded Genesis Admin Public Key (Ed25519)
// Corresponds to 'nbex1ckhh5p27wu4lee3qrppa8mt8lt0dvdxqr0an3hmhv2j0y80e86esk40mft' (Alice)
// This key allows spending from 'patrimonio:fees' and other system accounts.
pub const GENESIS_ADMIN_PK: &str = "8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c";
