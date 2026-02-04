use atlas_common::{
    error::{Result, AtlasError},
    entry::{Leg, LegKind, LedgerEntry},
    genesis::{GenesisState, GENESIS_ADMIN_PK}
};
use ed25519_dalek::{Verifier, VerifyingKey, Signature};
use crate::Ledger;

impl Ledger {
    /// Applies the genesis state if the ledger is empty.
    pub async fn apply_genesis_state(&self, genesis: &GenesisState) -> Result<()> {
        let mut state = self.state.write().await;
        
        // Fix: 'State::new' populates hardcoded Mint/Alice, so is_empty() is always false.
        // We check if "vault:genesis" exists to know if Genesis was applied.
        if state.accounts.contains_key("vault:genesis") {
            return Ok(());
        }

        // === PHASE 0: Calculate total supply needed for genesis ===
        let mut total_atlas_supply: u128 = 0;
        for (_address, amount) in &genesis.allocations {
            total_atlas_supply += *amount as u128;
        }
        
        // Add issuance reserve
        let issuance_reserve: u128 = 100_000_000u128 * 1_000_000u128; // 100M units * 6 decimals
        total_atlas_supply += issuance_reserve;
        
        // === PHASE 1: Pre-fund source vaults (so debits can happen) ===
        // vault:genesis is the source of all user allocations
        Self::credit_account(&mut state, "vault:genesis", crate::core::ledger::asset::ATLAS_FULL_ID, total_atlas_supply);
        
        // Pre-fund vault:mint for multi-asset allocations (USD, EUR, etc)
        // DOUBLE-ENTRY: vault:mint (Asset) must have counterpart in vault:capital (Equity)
        let mint_issuer = "wallet:mint";
        let multi_asset_supply: u128 = 100_000_000; // Enough for dev allocations
        for symbol in ["USD", "EUR", "GBP", "BRL", "XAU"] {
            let asset_id = format!("{}/{}", mint_issuer, symbol);
            let source_vault = format!("vault:mint:{}", symbol);
            let capital_vault = format!("vault:capital:{}", symbol);
            
            // Debit: Asset side (vault:mint holds the tokens)
            Self::credit_account(&mut state, &source_vault, &asset_id, multi_asset_supply);
            // Credit: Equity side (vault:capital is the source/authorized capital)
            Self::credit_account(&mut state, &capital_vault, &asset_id, multi_asset_supply);
        }
        
        // === PHASE 2: Allocate to user wallets (Debit source, Credit wallet) ===
        for (address, amount) in &genesis.allocations {
             let final_address = Self::resolve_genesis_address(address);
             let account_key = format!("wallet:{}", final_address);

              // 1. Debit Equity (Genesis Vault - reduces available supply)
              let debit_leg = Leg {
                  account: "vault:genesis".to_string(),
                  asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
                  kind: LegKind::Debit, 
                  amount: *amount as u128,
              };

             // 2. Credit Liability (User Wallet)
             let credit_leg = Leg {
                 account: account_key.clone(),
                 asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
                 kind: LegKind::Credit,
                 amount: *amount as u128,
             };

             // Apply to memory state (balanced)
             Self::debit_account(&mut state, "vault:genesis", crate::core::ledger::asset::ATLAS_FULL_ID, *amount as u128);
             Self::credit_account(&mut state, &account_key, crate::core::ledger::asset::ATLAS_FULL_ID, *amount as u128);
             
             // --- DEV/TESTNET: Inject Multi-Asset Allocations for specific addresses ---
             let mut extra_legs = Vec::new();
             if let Some(extras) = Self::get_dev_allocations(address) {
                 for (asset_id, val) in extras {
                     let asset_symbol = asset_id.split('/').last().unwrap_or("UNKNOWN");
                     let source_vault = format!("vault:mint:{}", asset_symbol);
                     
                     // Debit source vault, Credit user wallet
                     Self::debit_account(&mut state, &source_vault, &asset_id, val);
                     Self::credit_account(&mut state, &account_key, &asset_id, val);

                     extra_legs.push(Leg {
                         account: source_vault,
                         asset: asset_id.clone(),
                         kind: LegKind::Debit,
                         amount: val,
                     });
                     
                     extra_legs.push(Leg {
                         account: account_key.clone(),
                         asset: asset_id,
                         kind: LegKind::Credit,
                         amount: val,
                     });
                 }
             }

              // 3. Create Genesis Entry for Persistence
              let mut genesis_legs = vec![debit_leg, credit_leg]; 
              genesis_legs.extend(extra_legs);
 
              let entry = LedgerEntry::new(
                 format!("genesis-{}", final_address),
                 genesis_legs,
                 "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                 0,
                 0,
                 Some("GENESIS ALLOCATION".to_string()),
             );

             tracing::info!("🏛️ Applied Genesis (RAM): {} -> {} ATLAS (Key: {})", address, amount, account_key);

             // 4. Persist to Shards
             let shards = self.shards.read().await;
             if let Err(e) = shards.append(&account_key, &entry).await {
                 tracing::error!("❌ Failed to write Genesis shard for {}: {}", account_key, e);
             } else {
                 tracing::info!("💾 Persisted Genesis Shard for {}", account_key);
             }
        }

        // === PHASE 3: Setup Issuance Pool (for inflation) ===
        // vault:issuance holds authorized but unissued supply
        // When inflation happens: Debit vault:issuance, Credit vault:treasury/validators/users
        
        // Credit issuance pool from genesis source
        Self::debit_account(&mut state, "vault:genesis", crate::core::ledger::asset::ATLAS_FULL_ID, issuance_reserve);
        Self::credit_account(&mut state, "vault:issuance", crate::core::ledger::asset::ATLAS_FULL_ID, issuance_reserve);
        
        let issuance_entry = LedgerEntry::new(
            "genesis-issuance".to_string(),
            vec![
                Leg {
                    account: "vault:genesis".to_string(),
                    asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
                    kind: LegKind::Debit,
                    amount: issuance_reserve,
                },
                Leg {
                    account: "vault:issuance".to_string(),
                    asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
                    kind: LegKind::Credit,
                    amount: issuance_reserve,
                },
            ],
            "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            0,
            0,
            Some("GENESIS ISSUANCE POOL".to_string()),
        );

        let shards = self.shards.read().await;
        if let Err(e) = shards.append("vault:issuance", &issuance_entry).await {
            tracing::error!("❌ Failed to write Genesis Issuance shard: {}", e);
        } else {
            tracing::info!("💾 Persisted Genesis Issuance Shard");
        }
        
        // Init other vaults (empty, will accumulate via transactions)
        let _ = state.accounts.entry("vault:treasury".to_string()).or_insert_with(crate::core::ledger::account::AccountState::new);
        let _ = state.accounts.entry("vault:fees".to_string()).or_insert_with(crate::core::ledger::account::AccountState::new);

        Ok(())
    }

    /// Resolves generic address formats (Base58/PeerID) to standard Bech32 addresses.
    fn resolve_genesis_address(address: &str) -> String {
        if !address.starts_with("nbex") && !address.starts_with("nbhd") && !address.starts_with("0x") {
             if let Ok(bytes) = bs58::decode(address).into_vec() {
                 // Case 1: Raw Ed25519 Public Key (32 bytes)
                 if bytes.len() == 32 {
                     if let Ok(verifying_key) = VerifyingKey::from_bytes(bytes.as_slice().try_into().unwrap()) {
                         if let Ok(bech32) = atlas_common::address::address::Address::address_from_pk(&verifying_key, "nbex") {
                             tracing::info!("🔄 Genesis Migration: Converted Raw Key {} -> {}", address, bech32);
                             return bech32;
                         }
                     }
                 }
                 // Case 2: Libp2p PeerID (38 bytes: [0x00, 0x24, 0x08, 0x01, ...])
                 else if bytes.len() == 38 && bytes.starts_with(&[0x00, 0x24, 0x08, 0x01, 0x12, 0x20]) {
                     let pub_key_bytes = &bytes[6..];
                     if let Ok(verifying_key) = VerifyingKey::from_bytes(pub_key_bytes.try_into().unwrap()) {
                          if let Ok(bech32) = atlas_common::address::address::Address::address_from_pk(&verifying_key, "nbex") {
                             tracing::info!("🔄 Genesis Migration: Converted PeerID {} -> {}", address, bech32);
                             return bech32;
                         }
                     }
                 }
             }
        }
        address.to_string()
    }

    /// Returns development/testnet allocations for specific addresses.
    fn get_dev_allocations(address: &str) -> Option<Vec<(String, u128)>> {
        if address.starts_with("nbex1ck") || address.starts_with("nbhd1k") {
            let mint_issuer = "wallet:mint";
            Some(vec![
                (format!("{}/USD", mint_issuer), 5_000),
                (format!("{}/EUR", mint_issuer), 10_000),
                (format!("{}/GBP", mint_issuer), 10_000),
                (format!("{}/BRL", mint_issuer), 10_000),
                (format!("{}/XAU", mint_issuer), 1_000),
            ])
        } else {
            None
        }
    }

    /// Helper to credit (add to) account balance in memory.
    fn credit_account(state: &mut crate::core::ledger::state::State, account: &str, asset: &str, amount: u128) {
        let account_state = state.accounts.entry(account.to_string()).or_insert_with(crate::core::ledger::account::AccountState::new);
        let balance = account_state.balances.entry(asset.to_string()).or_insert(0);
        *balance += amount;
    }
    
    /// Helper to debit (subtract from) account balance in memory.
    fn debit_account(state: &mut crate::core::ledger::state::State, account: &str, asset: &str, amount: u128) {
        let account_state = state.accounts.entry(account.to_string()).or_insert_with(crate::core::ledger::account::AccountState::new);
        let balance = account_state.balances.entry(asset.to_string()).or_insert(0);
        *balance = balance.saturating_sub(amount);
    }
}
