use std::collections::HashMap;
use sha2::{Digest, Sha256};
use atlas_common::entry::{LedgerEntry, LegKind};
use crate::core::ledger::account::AccountState;
use crate::core::ledger::asset::AssetDefinition;


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
    pub assets: HashMap<String, AssetDefinition>, // Key: "Issuer/Symbol"
    pub institutions: atlas_bank::institution_core::registry::InstitutionRegistry,
}

impl State {
    pub fn new() -> Self {
        let mut accounts = HashMap::new();
        let mut assets = HashMap::new();
        
        // Define Central Mint Issuer
        let mint_issuer = crate::core::ledger::asset::SYSTEM_MINT_ISSUER.to_string();

        // Register default assets
        let usd = AssetDefinition::new(
            mint_issuer.clone(),
            // AssetType::L2_1_3, // REMOVED
            "US Dollar".to_string(),
            "USD".to_string(),
            2,
            Some("ISO4217:USD".to_string()),
        );
        assets.insert(usd.id(), usd);

        let brl = AssetDefinition::new(
            mint_issuer.clone(),
            // AssetType::L2_1_3, // REMOVED
            "Brazilian Real".to_string(),
            "BRL".to_string(),
            2,
            Some("ISO4217:BRL".to_string()),
        );
        assets.insert(brl.id(), brl);

        let gbp = AssetDefinition::new(
            mint_issuer.clone(),
            // AssetType::L2_1_3,
            "British Pound".to_string(),
            "GBP".to_string(),
            2,
            Some("ISO4217:GBP".to_string()),
        );
        assets.insert(gbp.id(), gbp);

        let eur = AssetDefinition::new(
            mint_issuer.clone(),
            // AssetType::L2_1_3,
            "Euro".to_string(),
            "EUR".to_string(),
            2,
            Some("ISO4217:EUR".to_string()),
        );
        assets.insert(eur.id(), eur);

        let gold = AssetDefinition::new(
            mint_issuer.clone(),
            // AssetType::A1_2_3, 
            "Physical Gold (99.9%)".to_string(),
            "XAU".to_string(),
            4, 
            Some("Commodity:XAU".to_string()), 
        );
        assets.insert(gold.id(), gold);

        let atlas = AssetDefinition::new(
            mint_issuer.clone(),
            // AssetType::EQ3_1, // Equity
            "Atlas Token".to_string(),
            crate::core::ledger::asset::ATLAS_SYMBOL.to_string(),
            8,
            None,
        );
        assets.insert(atlas.id(), atlas);

        // Genesis: Mint Account
        // Mint holds the supply? Or is it a liability?
        // For Liability (L2_1), the Mint account should technically have a Negative balance or be the issuer.
        // For simplicity in this non-double-entry-enforced-genesis, we give Mint 0.
        // NOTE: In strict accounting, Mint has 0, creating money debits Cash (Assets) and credits Reserve (Liabilities).
        // Here we just seed balances.
        let mint = AccountState::new();
        // mint.balances.insert(format!("{}/USD", mint_issuer), 1_000_000); // FIXED: Removed hardcoded 1M
        accounts.insert("mint".to_string(), mint);

        // Genesis: Issuance Vault (Authorized Capital)
        // FIXED: Start empty. Balances are seeded via apply_genesis_state() with proper double-entry.
        accounts.insert("vault:issuance".to_string(), AccountState::new());

        // Genesis: Treasury Vault
        accounts.insert("vault:treasury".to_string(), AccountState::new());
        
        // Genesis: Fees Vault (System Revenue)
        accounts.insert("vault:fees".to_string(), AccountState::new());
        
        // Genesis: Unissued Vault (Authorized but Unissued Capital)
        accounts.insert("vault:unissued".to_string(), AccountState::new());

        // Genesis: User Wallets are created on-demand or via apply_genesis_state()
        // REMOVED: Hardcoded balances that violated double-entry accounting

        Self {
            accounts,
            delegations: DelegationStore::new(),
            assets,
            institutions: atlas_bank::institution_core::registry::InstitutionRegistry::new(),
        }
    }

    /// Applies a LedgerEntry to the state.
    /// Validates that Debits == Credits for each asset.
    /// Applies a LedgerEntry to the state atomically.
    /// Follows a Two-Phase Commit strategy:
    /// Phase 1: Simulate & Validate (Double-Entry + Balances)
    /// Phase 2: Apply Changes (Memory Mutation)
    pub fn apply_entry(&mut self, entry: LedgerEntry) -> Result<(), String> {
        // --- PHASE 1: Validation (ReadOnly) ---
        
        // 1.0 Validate Asset Existence
        for leg in &entry.legs {
            if !self.assets.contains_key(&leg.asset) {
                return Err(format!("Asset currently not registered in Ledger: '{}'. Assets must be strictly defined before usage.", leg.asset));
            }
        }

        // 1.1 Validate Double-Entry Rule (Debits == Credits)
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
                return Err(format!("Accounting Error: Unbalanced entry for asset {}: net {}", asset, total));
            }
        }

        // 1.2 Validate Balances (Simulation)
        // We must check if accounts have enough funds for Debits WITHOUT modifying state yet.
        for leg in &entry.legs {
            if let LegKind::Debit = leg.kind {
                // Peek account
                let balance = if let Some(account) = self.accounts.get(&leg.account) {
                    *account.balances.get(&leg.asset).unwrap_or(&0)
                } else {
                    0 // Account doesn't exist -> Balance 0
                };

                if balance < leg.amount {
                    return Err(format!("Insufficient funds for account {} asset {} (Required: {}, Available: {})", 
                        leg.account, leg.asset, leg.amount, balance));
                }
            }
        }

        // --- PHASE 2: Execution (Mutation) ---
        // At this point, all checks passed. We can safely mutate.
        // This phase SHOULD NOT fail via Result (logic errors only).

        let mut involved_accounts = std::collections::HashSet::new();

        for leg in entry.legs {
            let account = self.accounts.entry(leg.account.clone()).or_insert_with(AccountState::new);
            
            let balance = account.balances.entry(leg.asset.clone()).or_insert(0);
            match leg.kind {
                LegKind::Debit => *balance -= leg.amount,
                LegKind::Credit => *balance += leg.amount,
            }
            involved_accounts.insert(leg.account);
        }

        // Update Headers (ONCE per account)
        for acc_id in involved_accounts {
            if let Some(account) = self.accounts.get_mut(&acc_id) {
                account.last_entry_id = Some(entry.entry_id.clone());
                account.last_transaction_hash = Some(entry.tx_hash.clone());
                // Nonce update is handled by Transaction Engine for the Sender only.
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_common::entry::{Leg, LegKind, LedgerEntry};

    #[test]
    fn test_atomic_commit_reverts_on_failure() {
        let mut state = State::new();
        // Setup: Give Alice 100
        state.accounts.entry("Alice".to_string()).or_insert_with(AccountState::new)
            .balances.insert("wallet:mint/USD".to_string(), 100);

        // Transaction: Alice -> Bob 150 (Should fail due to insufficient funds)
        // Leg 1: Debit Alice 150 (Fails Phase 1)
        // Leg 2: Credit Bob 150
        let legs = vec![
            Leg { account: "Alice".to_string(), asset: "wallet:mint/USD".to_string(), kind: LegKind::Debit, amount: 150 },
            Leg { account: "Bob".to_string(), asset: "wallet:mint/USD".to_string(), kind: LegKind::Credit, amount: 150 },
        ];
        let entry = LedgerEntry::new("tx1".to_string(), legs, "hash1".to_string(), 0, 0, None);

        let result = state.apply_entry(entry);
        assert!(result.is_err());
        
        // ASSERT ATOMICITY: Alice should still have 100, NOT -50 or 100 but Bob having 150.
        // And Bob should not exist or have 0.
        let alice_bal = *state.accounts.get("Alice").unwrap().balances.get("wallet:mint/USD").unwrap();
        assert_eq!(alice_bal, 100); 
        assert!(state.accounts.get("Bob").is_none());
    }

    #[test]
    fn test_double_entry_enforcement() {
        let mut state = State::new();
        // Transaction: Mint 100 USD to Alice but forget to credit liability (Unbalanced)
        let legs = vec![
            Leg { account: "Alice".to_string(), asset: "wallet:mint/USD".to_string(), kind: LegKind::Credit, amount: 100 },
        ];
        let entry = LedgerEntry::new("tx2".to_string(), legs, "hash2".to_string(), 0, 0, None);

        let result = state.apply_entry(entry);
        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Unbalanced"));
    }

    #[test]
    fn test_aec_chaining_pointers_update() {
        let mut state = State::new();
        
        // Tx 1: Equity -> Alice 100
        let legs = vec![
            Leg { account: "Equity".to_string(), asset: "wallet:mint/USD".to_string(), kind: LegKind::Debit, amount: 100 },
            Leg { account: "Alice".to_string(), asset: "wallet:mint/USD".to_string(), kind: LegKind::Credit, amount: 100 },
        ];
        // Hack: Manually fund Equity for the test or disable balance check for Equity? 
        // Our Phase 1 checks balance. So let's fund Equity first.
        state.accounts.entry("Equity".to_string()).or_insert_with(AccountState::new)
            .balances.insert("wallet:mint/USD".to_string(), 1000);

        let entry1 = LedgerEntry::new("tx1".to_string(), legs, "params_hash_1".to_string(), 0, 0, None);
        state.apply_entry(entry1).unwrap();

        // Verify Alice Pointer
        let alice = state.accounts.get("Alice").unwrap();
        assert_eq!(alice.last_transaction_hash, Some("params_hash_1".to_string()));

        // Tx 2: Alice -> Bob 50
        let legs2 = vec![
            Leg { account: "Alice".to_string(), asset: "wallet:mint/USD".to_string(), kind: LegKind::Debit, amount: 50 },
            Leg { account: "Bob".to_string(), asset: "wallet:mint/USD".to_string(), kind: LegKind::Credit, amount: 50 },
        ];
        let entry2 = LedgerEntry::new("tx2".to_string(), legs2, "params_hash_2".to_string(), 0, 0, None);
        state.apply_entry(entry2).unwrap();

        // Verify Alice Pointer Moved
        let alice_v2 = state.accounts.get("Alice").unwrap();
        assert_eq!(alice_v2.last_transaction_hash, Some("params_hash_2".to_string()));
        
        // Verify Bob Pointer Created
        let bob = state.accounts.get("Bob").unwrap();
        assert_eq!(bob.last_transaction_hash, Some("params_hash_2".to_string()));
    }
}
