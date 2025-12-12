pub mod entry;
pub mod account;

use std::collections::HashMap;
use sha2::{Digest, Sha256};
use self::entry::{LedgerEntry, LegKind};
use self::account::AccountState;

/// Represents the global state of the application.
/// Now follows FIP-02: Double-Entry Accounting.
#[derive(Debug, Clone, Default)]
pub struct State {
    pub accounts: HashMap<String, AccountState>,
}

impl State {
    pub fn new() -> Self {
        let mut accounts = HashMap::new();
        // Genesis: Mint account with funds
        let mut mint = AccountState::new();
        mint.balances.insert("USD".to_string(), 1_000_000);
        accounts.insert("mint".to_string(), mint);

        Self {
            accounts,
        }
    }

    /// Applies a LedgerEntry to the state.
    /// Validates that Debits == Credits for each asset.
    pub fn apply_entry(&mut self, entry: LedgerEntry) -> Result<(), String> {
        // 1. Validate Double-Entry Rule (Debits == Credits)
        let mut asset_totals: HashMap<String, i128> = HashMap::new();

        for leg in &entry.legs {
            let amount = leg.amount as i128;
            let entry = asset_totals.entry(leg.asset.clone()).or_insert(0);
            match leg.kind {
                LegKind::Debit => *entry -= amount,
                LegKind::Credit => *entry += amount,
            }
        }

        for (asset, total) in asset_totals {
            if total != 0 {
                return Err(format!("Unbalanced entry for asset {}: net {}", asset, total));
            }
        }

        // 2. Apply changes
        for leg in entry.legs {
            let account = self.accounts.entry(leg.account.clone()).or_insert_with(AccountState::new);
            
            let balance = account.balances.entry(leg.asset.clone()).or_insert(0);
            match leg.kind {
                LegKind::Debit => {
                    if *balance < leg.amount {
                        return Err(format!("Insufficient funds for account {} asset {}", leg.account, leg.asset));
                    }
                    *balance -= leg.amount;
                },
                LegKind::Credit => *balance += leg.amount,
            }
            
            account.last_entry_id = Some(entry.entry_id.clone());
        }

        Ok(())
    }

    /// Generates the leaves for the Merkle Tree.
    /// Leaves are Hash(account_id + account_state_hash), sorted by account_id.
    pub fn get_leaves(&self) -> Vec<Vec<u8>> {
        let mut keys: Vec<&String> = self.accounts.keys().collect();
        keys.sort();

        keys.iter().map(|k| {
            let mut hasher = Sha256::new();
            hasher.update(k.as_bytes());
            if let Some(account) = self.accounts.get(*k) {
                // Simple serialization for hash
                let bytes = bincode::serialize(account).unwrap_or_default();
                hasher.update(bytes);
            }
            hasher.finalize().to_vec()
        }).collect()
    }
}
