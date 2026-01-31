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
        
        for (address, amount) in &genesis.allocations {
             let final_address = Self::resolve_genesis_address(address);

              // 1. Debit Equity (Issuance)
              let debit_leg = Leg {
                  account: "vault:genesis".to_string(), // Equity/Vault
                  asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
                  kind: LegKind::Debit, 
                  amount: *amount as u128,
              };

             // 2. Credit Liability (User Wallet)
             let account_key = format!("wallet:{}", final_address);

             let credit_leg = Leg {
                 account: account_key.clone(),
                 asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
                 kind: LegKind::Credit, // Increases Liability
                 amount: *amount as u128,
             };

             // Double Entry Bypass for Genesis (Creation)
             
             // 1. Update/Create Wallet Account (ATLAS)
             Self::update_account_balance(&mut state, &account_key, crate::core::ledger::asset::ATLAS_FULL_ID, *amount as u128);
             
             // --- DEV/TESTNET: Inject Multi-Asset Allocations for specific addresses ---
             let mut extra_legs = Vec::new();
             if let Some(extras) = Self::get_dev_allocations(address) {
                 for (asset_id, val) in extras {
                     // Phase 1: Update Wallet (Credit User)
                     Self::update_account_balance(&mut state, &account_key, &asset_id, val);

                     // Phase 2: Update Treasury (Credit System Reserve)
                     Self::update_account_balance(&mut state, "vault:treasury", &asset_id, val);

                     // Phase 3: Record Legs
                     extra_legs.push(Leg {
                         account: account_key.clone(),
                         asset: asset_id.clone(),
                         kind: LegKind::Credit,
                         amount: val,
                     });
                     
                     extra_legs.push(Leg {
                         account: "vault:treasury".to_string(),
                         asset: asset_id,
                         kind: LegKind::Credit, // Credit Treasury (Asset)
                         amount: val,
                     });
                 }
             }
             // --------------------------------------------------------------------------
             
             // 2. Update/Create Equity Account (The Source)
             Self::update_account_balance(&mut state, "vault:genesis", crate::core::ledger::asset::ATLAS_FULL_ID, *amount as u128);

              // 3. Create Genesis Entry for Persistence
              let mut genesis_legs = vec![debit_leg, credit_leg]; 
              genesis_legs.extend(extra_legs);
 
              let entry = LedgerEntry::new(
                 format!("genesis-{}", final_address),
                 genesis_legs, // All legs
                 "0000000000000000000000000000000000000000000000000000000000000000".to_string(), // Genesis Hash
                 0,
                 0,
                 Some("GENESIS ALLOCATION".to_string()),
             );

             tracing::info!("🏛️ Applied Genesis (RAM): {} -> {} ATLAS (Key: {})", address, amount, account_key);

             // 4. Persist to Shards (Explicitly generate FILE)
             let shards = self.shards.read().await;
             if let Err(e) = shards.append(&account_key, &entry).await {
                 tracing::error!("❌ Failed to write Genesis shard for {}: {}", account_key, e);
             } else {
                 tracing::info!("💾 Persisted Genesis Shard for {}", account_key);
             }
        }

        // --- PRE-FUND INFLATION POOLS (Singleton, Outside Loop) ---
        // Authorized Supply (Unissued).
        let issuance_reserve: u128 = 100_000_000u128 * 1_000_000u128; // 100M units * 6 decimals
        
        // 1. Credit Issuance (Equity side - Authorization)
        Self::update_account_balance(&mut state, "vault:issuance", crate::core::ledger::asset::ATLAS_FULL_ID, issuance_reserve);
        let issuance_credit = Leg {
              account: "vault:issuance".to_string(),
              asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
              kind: LegKind::Credit, // Increase Authorized Supply
              amount: issuance_reserve,
        };

        // 2. Debit Unissued (Asset side - Potential)
        // This balances the equation: Ativo (Unissued) = PL (Capital Social Autorizado)
        Self::update_account_balance(&mut state, "vault:unissued", crate::core::ledger::asset::ATLAS_FULL_ID, issuance_reserve);
        let issuance_debit = Leg {
              account: "vault:unissued".to_string(),
              asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
              kind: LegKind::Debit, // Increase Asset
              amount: issuance_reserve,
        };
        
        // Persist Issuance (Double Entry)
        let issuance_entry = LedgerEntry::new(
            "genesis-issuance".to_string(),
            vec![issuance_debit, issuance_credit], 
            "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            0,
            0,
            Some("GENESIS ISSUANCE".to_string()),
        );

        let shards = self.shards.read().await;
        if let Err(e) = shards.append("vault:issuance", &issuance_entry).await {
            tracing::error!("❌ Failed to write Genesis Issuance shard: {}", e);
        } else {
            tracing::info!("💾 Persisted Genesis Issuance Shard");
        }
        
        // Also init Treasury just in case
        let _treasury_account = state.accounts.entry("vault:treasury".to_string()).or_insert_with(crate::core::ledger::account::AccountState::new);

        
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

    /// Helper to update account balance in memory.
    fn update_account_balance(state: &mut crate::core::ledger::state::State, account: &str, asset: &str, amount: u128) {
        let account_state = state.accounts.entry(account.to_string()).or_insert_with(crate::core::ledger::account::AccountState::new);
        let balance = account_state.balances.entry(asset.to_string()).or_insert(0);
        *balance += amount;
    }
}
