
use std::collections::HashMap;
use sha2::{Digest, Sha256};
use atlas_common::entry::{LedgerEntry, LegKind};
use crate::core::ledger::account::AccountState;

/// Stores delegation information.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DelegationStore {
    // Delegator -> Validator -> Amount
    pub delegations: HashMap<String, HashMap<String, u64>>,
    // Validator -> Total Delegated Power
    pub validator_power: HashMap<String, u64>,
}

impl DelegationStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn delegate(&mut self, delegator: String, validator: String, amount: u64) {
        let user_delegations = self.delegations.entry(delegator).or_default();
        *user_delegations.entry(validator.clone()).or_default() += amount;
        
        *self.validator_power.entry(validator).or_default() += amount;
    }

    pub fn undelegate(&mut self, delegator: String, validator: String, amount: u64) -> Result<(), String> {
        let user_delegations = self.delegations.entry(delegator.clone()).or_default();
        let current = user_delegations.entry(validator.clone()).or_default();
        
        if *current < amount {
            return Err(format!("Insufficient delegation: has {}, tried to undelegate {}", current, amount));
        }
        
        *current -= amount;
        if *current == 0 {
            user_delegations.remove(&validator);
        }
        if user_delegations.is_empty() {
            self.delegations.remove(&delegator);
        }

        let val_power = self.validator_power.entry(validator).or_default();
        *val_power = val_power.saturating_sub(amount); // Safety

        Ok(())
    }
    
    pub fn get_delegated_power(&self, validator: &str) -> u64 {
        *self.validator_power.get(validator).unwrap_or(&0)
    }

    /// Slashes all delegators of a given validator by a percentage.
    /// Returns the total amount slashed.
    pub fn slash_delegators(&mut self, validator: &str, percentage: u8) -> u64 {
        let mut total_slashed = 0;
        let mut removed_delegators = Vec::new();

        // Iterate over all delegations (This is O(N) where N = total delegators in system. suboptimal but works for now)
        // Optimization: Store reverse map Validator -> Vec<Delegator> in future.
        for (delegator, investments) in self.delegations.iter_mut() {
            if let Some(amount) = investments.get_mut(validator) {
                let penalty = (*amount * percentage as u64) / 100;
                if penalty > 0 {
                    *amount -= penalty;
                    total_slashed += penalty;
                    
                    if *amount == 0 {
                        investments.remove(validator);
                        if investments.is_empty() {
                            removed_delegators.push(delegator.clone());
                        }
                    }
                }
            }
        }

        // Clean up empty delegators
        for del in removed_delegators {
            self.delegations.remove(&del);
        }

        // Update validator power
        let val_power = self.validator_power.entry(validator.to_string()).or_default();
        *val_power = val_power.saturating_sub(total_slashed);

        total_slashed
    }
}

/// Represents the global state of the application.
/// Now follows FIP-02: Double-Entry Accounting.
#[derive(Debug, Clone, Default)]
pub struct State {
    pub accounts: HashMap<String, AccountState>,
    pub delegations: DelegationStore,
}

impl State {
    pub fn new() -> Self {
        let mut accounts = HashMap::new();
        let mut mint = AccountState::new();
        mint.balances.insert("USD".to_string(), 1_000_000);
        mint.balances.insert("BRL".to_string(), 1_000_000); // System Mint BRL
        accounts.insert("mint".to_string(), mint);

        // Genesis: User Wallet (Exposed - nbex)
        let mut wallet_alice_exposed = AccountState::new();
        wallet_alice_exposed.balances.insert("BRL".to_string(), 5_000);
        wallet_alice_exposed.balances.insert("MOX".to_string(), 10_000);
        accounts.insert("passivo:wallet:nbex1ckhh5p27wu4lee3qrppa8mt8lt0dvdxqr0an3hmhv2j0y80e86esk40mft".to_string(), wallet_alice_exposed);

        // Genesis: User Wallet (Hidden - nbhd)
        let mut wallet_alice_hidden = AccountState::new();
        wallet_alice_hidden.balances.insert("BRL".to_string(), 5_000);
        wallet_alice_hidden.balances.insert("MOX".to_string(), 10_000);
        accounts.insert("passivo:wallet:nbhd1k7magn8v7jpqk96xvdnquwl4xsgmnnknkqsgrrk35g6ascx7fqks893gps".to_string(), wallet_alice_hidden);

        Self {
            accounts,
            delegations: DelegationStore::new(),
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
