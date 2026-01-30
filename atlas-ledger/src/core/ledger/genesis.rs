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
        // We check if "patrimonio:genesis" exists to know if Genesis was applied.
        if state.accounts.contains_key("patrimonio:genesis") {
            return Ok(());
        }
        
        for (address, amount) in &genesis.allocations {
             // Fix: Convert Base58 (Genesis standard) to Bech32 (Ledger standard)
             // If address doesn't start with nbex or 0x, try to parse as Base58 public key.
             let mut final_address = address.clone();
             
             if !address.starts_with("nbex") && !address.starts_with("nbhd") && !address.starts_with("0x") {
                 if let Ok(bytes) = bs58::decode(address).into_vec() {
                     // Case 1: Raw Ed25519 Public Key (32 bytes)
                     if bytes.len() == 32 {
                         if let Ok(verifying_key) = VerifyingKey::from_bytes(bytes.as_slice().try_into().unwrap()) {
                             if let Ok(bech32) = atlas_common::address::address::Address::address_from_pk(&verifying_key, "nbex") {
                                 final_address = bech32;
                                 tracing::info!("🔄 Genesis Migration: Converted Raw Key {} -> {}", address, final_address);
                             }
                         }
                     }
                     // Case 2: Libp2p PeerID (38 bytes: [0x00, 0x24, 0x08, 0x01, ...])
                     // Identity pattern: code(0x00) + len(0x24=36) + key_type(0x0801=Ed25519) + key_len(0x1220=32 bytes)
                     else if bytes.len() == 38 && bytes.starts_with(&[0x00, 0x24, 0x08, 0x01, 0x12, 0x20]) {
                         let pub_key_bytes = &bytes[6..];
                         if let Ok(verifying_key) = VerifyingKey::from_bytes(pub_key_bytes.try_into().unwrap()) {
                              if let Ok(bech32) = atlas_common::address::address::Address::address_from_pk(&verifying_key, "nbex") {
                                 final_address = bech32;
                                 tracing::info!("🔄 Genesis Migration: Converted PeerID {} -> {}", address, final_address);
                             }
                         }
                     }
                 }
             }

              // 1. Debit Equity (Issuance)
              let _debit_leg = Leg {
                  account: "patrimonio:genesis".to_string(), // Equity
                  asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
                  kind: LegKind::Debit, // Reduces Equity (Technically Equity is Credit normal, so Debit reduces it to create Liability)
                  amount: *amount as u128,
              };

             // 2. Credit Liability (User Wallet)
             let account_key = if final_address.starts_with("nbex") || final_address.starts_with("nbhd") {
                 format!("passivo:wallet:{}", final_address) // STANDARD: Wrap with prefix
             } else {
                 format!("passivo:wallet:{}", final_address) // Wrap legacy
             };

             let credit_leg = Leg {
                 account: account_key.clone(),
                 asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
                 kind: LegKind::Credit, // Increases Liability
                 amount: *amount as u128,
             };

             // Double Entry Bypass for Genesis (Creation)
             
             // 1. Update/Create Wallet Account (ATLAS)
             {
                 let wallet_account = state.accounts.entry(account_key.clone()).or_insert_with(crate::core::ledger::account::AccountState::new);
                 let balance = wallet_account.balances.entry(crate::core::ledger::asset::ATLAS_FULL_ID.to_string()).or_insert(0);
                 *balance += *amount as u128; // Credit
             }
             
             // --- DEV/TESTNET: Inject Multi-Asset Allocations for specific addresses ---
             // To fix current testing, we inject these "bonus" assets here AND ensure they are part of the shard structure if we want full persistence.
             
             let mut extra_legs = Vec::new(); // defined outside scopes

             if address.starts_with("nbex1ck") || address.starts_with("nbhd1k") {
                    let mint_issuer = "passivo:wallet:mint";
                    let extras = vec![
                        (format!("{}/USD", mint_issuer), 5_000),
                        (format!("{}/EUR", mint_issuer), 10_000),
                        (format!("{}/GBP", mint_issuer), 10_000),
                        (format!("{}/BRL", mint_issuer), 10_000),
                        (format!("{}/XAU", mint_issuer), 1_000),
                    ];

                    // Phase 1: Update Wallet (Credit User)
                    {
                        let wallet_account = state.accounts.entry(account_key.clone()).or_insert_with(crate::core::ledger::account::AccountState::new);
                         for (asset_id, val) in &extras {
                             let b = wallet_account.balances.entry(asset_id.clone()).or_insert(0);
                             *b += *val as u128;
                         }
                    }

                    // Phase 2: Update Treasury (Credit System Reserve)
                    // Instead of Debiting Issuance (which requires pre-fund and creates imbalance),
                    // We Credit Treasury. This simulates that the System HOLDS the matching asset (Backing)
                    // or that the "Minted" supply is split between User and Treasury?
                    // No, to balance "User has 20k" (Right), "System must have 20k" (Left).
                    // So we give Treasury 20k.
                    {
                        let treasury_account = state.accounts.entry("patrimonio:treasury".to_string()).or_insert_with(crate::core::ledger::account::AccountState::new);
                         for (asset_id, val) in &extras {
                             let t_bal = treasury_account.balances.entry(asset_id.clone()).or_insert(0);
                             *t_bal += *val as u128;
                         }
                    }

                    // Phase 3: Record Legs
                 for (asset_id, val) in extras {
                     extra_legs.push(Leg {
                         account: account_key.clone(),
                         asset: asset_id.clone(),
                         kind: LegKind::Credit,
                         amount: val as u128,
                     });
                     
                     extra_legs.push(Leg {
                         account: "patrimonio:treasury".to_string(),
                         asset: asset_id,
                         kind: LegKind::Credit, // Credit Treasury (Asset)
                         amount: val as u128,
                     });
                 }
             }
             // --------------------------------------------------------------------------
             
             // 2. Update/Create Equity Account (The Source)
             let equity_account = state.accounts.entry("patrimonio:genesis".to_string()).or_insert_with(crate::core::ledger::account::AccountState::new);
             let eq_balance = equity_account.balances.entry("ATLAS".to_string()).or_insert(0);
             *eq_balance += *amount as u128; 

                  let debit_leg = Leg {
                      account: "patrimonio:genesis".to_string(),
                      asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
                      kind: LegKind::Debit, 
                      amount: *amount as u128,
                  };

              // --- PRE-FUND INFLATION POOLS ---
              // To allow Debiting "patrimonio:issuance" (Minting), we must initialize it with a high balance (Authorized Supply).
              // Otherwise, Debits (reducing Equity) on a 0 balance will fail/underflow.
              // We create a "Reserve" of 1 Trillion ATLAS.
              
              let issuance_reserve: u128 = 1_000_000_000_000 * 1_000_000;
              let issuance_account = state.accounts.entry("patrimonio:issuance".to_string()).or_insert_with(crate::core::ledger::account::AccountState::new);
              let iss_bal = issuance_account.balances.entry(crate::core::ledger::asset::ATLAS_FULL_ID.to_string()).or_insert(0);
              *iss_bal += issuance_reserve;

              let issuance_credit = Leg {
                   account: "patrimonio:issuance".to_string(),
                   asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
                   kind: LegKind::Credit, // Credit Equity = Increase Reserve
                   amount: issuance_reserve,
              };

              // Also init Treasury just in case
               let _treasury_account = state.accounts.entry("patrimonio:treasury".to_string()).or_insert_with(crate::core::ledger::account::AccountState::new);

              // 3. Create Genesis Entry for Persistence
              let mut genesis_legs = vec![debit_leg, credit_leg, issuance_credit];
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
        
        Ok(())
    }
}
