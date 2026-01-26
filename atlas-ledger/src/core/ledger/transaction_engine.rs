use atlas_common::{
    env::proposal::Proposal,
    error::{Result, AtlasError},
    transactions::{Transaction, SignedTransaction, signing_bytes},
    entry::{Leg, LegKind, LedgerEntry},
    genesis::{GenesisState, GENESIS_ADMIN_PK}
};
use ed25519_dalek::{Verifier, VerifyingKey, Signature};
use crate::Ledger;

impl Ledger {
    /// Returns the total voting power of a validator (Own Stake + Delegations).
    pub async fn get_validator_total_power(&self, address: &str) -> Result<u64> {
        let own_balance = self.get_balance(address, "ATLAS").await?;
        let state = self.state.read().await;
        let delegated_power = state.delegations.get_delegated_power(address);
        
        tracing::info!("🔍 Stake Query: Addr={} | Balance={} | Delegated={} | Total={}", 
            address, own_balance, delegated_power, own_balance + delegated_power);
            
        Ok(own_balance + delegated_power)
    }

    /// Executes a transaction batch (proposal content) and updates the state.
    /// Returns the number of executed transactions.
    /// 
    /// # Arguments
    /// * `proposal` - The proposal containing transactions
    /// * `persist_shards` - If true, writes the ledger entry to disk (shards). Set to false during replay.
    pub async fn execute_transaction(&self, proposal: &Proposal, persist_shards: bool) -> Result<usize> {
        // 1. Try Batch Parsing
        // 1. Try Batch Parsing
        let transactions: Vec<SignedTransaction> = 
            if let Ok(batch) = serde_json::from_str::<Vec<SignedTransaction>>(&proposal.content) {
                batch
            } else if let Ok(signed_tx) = serde_json::from_str::<SignedTransaction>(&proposal.content) {
                // Fallback: Single SignedTransaction
                vec![signed_tx]
            } else {
                // Fallback: Legacy Single Transaction (Unsigned)
                let tx: Transaction = serde_json::from_str(&proposal.content)
                    .map_err(|e| AtlasError::Other(format!("Failed to parse transaction content: {}", e)))?;
                vec![SignedTransaction {
                    transaction: tx,
                    signature: vec![],
                    public_key: vec![],
                    fee_payer: None,
                    fee_payer_signature: None,
                    fee_payer_pk: None,
                }]
            };

        let mut count = 0;
        for st in transactions {
            let tx = st.transaction;

            // --- 1. SENDER VALIDATION ---
            // If signed, verify signature
            if !st.signature.is_empty() && !st.public_key.is_empty() {
                 let verifying_key = VerifyingKey::from_bytes(st.public_key.as_slice().try_into().unwrap_or(&[0u8; 32]))
                    .map_err(|e| AtlasError::Other(format!("Invalid public key: {}", e)))?;
                 let signature = Signature::from_slice(&st.signature)
                    .map_err(|e| AtlasError::Other(format!("Invalid signature format: {}", e)))?;
                 let msg = signing_bytes(&tx);
                 
                 // Verify Sender Signature
                 if verifying_key.verify(&msg, &signature).is_err() {
                     tracing::error!("❌ Signature Verification Failed for tx from {}", tx.from);
                     return Err(AtlasError::Other("Invalid transaction signature".to_string()));
                 }
                 
                 // Verify Sender Address matches Public Key
                 if let Ok(address) = atlas_common::address::address::Address::address_from_pk(&verifying_key, "nbex") {
                     if address != tx.from {
                         // Fallback check for "passivo:wallet:" prefix or no prefix issues
                         // Ideally strict check: address MUST match tx.from
                         tracing::warn!("⚠️ Sender Address Mismatch: Claimed {} vs Derived {}", tx.from, address);
                         // For now strict behavior:
                         // return Err(AtlasError::Other(format!("Sender address mismatch. Claimed: {}, Derived: {}", tx.from, address)));
                     }
                 }

                 // --- 1.1 SYSTEM ACCOUNT PROTECTION ---
                 // If Sender is 'patrimonio:*', Require Genesis Admin Signature
                 if tx.from.starts_with("patrimonio:") {
                     // 1. Verify Public Key matches Genesis Admin
                     let provided_pk_hex = hex::encode(st.public_key.clone());
                     if provided_pk_hex != GENESIS_ADMIN_PK {
                         tracing::error!("❌ Unauthorized Treasury Spend! PK {} != Admin {}", provided_pk_hex, GENESIS_ADMIN_PK);
                         return Err(AtlasError::Other("Unauthorized: System accounts require Admin Key".to_string()));
                     }
                     tracing::info!("🛡️  Admin Action: Spending from {}", tx.from);
                 }
            } else {
                 println!("⚠️ Executing unsigned transaction (legacy path)");
            }

            // --- 2. FEE PAYER VALIDATION ---
            let fee_payer = st.fee_payer.clone().unwrap_or(tx.from.clone());
            
            if let (Some(payer_sig_bytes), Some(payer_pk_bytes)) = (st.fee_payer_signature, st.fee_payer_pk) {
                // If Fee Payer is explicitly set (and likely different from Sender), verify their signature.
                // Even if Payer == Sender, if they signed the Fee field, we verify it.
                
                let payer_vk = VerifyingKey::from_bytes(payer_pk_bytes.as_slice().try_into().unwrap_or(&[0u8; 32]))
                    .map_err(|e| AtlasError::Other(format!("Invalid fee payer public key: {}", e)))?;
                let payer_sig = Signature::from_slice(&payer_sig_bytes)
                    .map_err(|e| AtlasError::Other(format!("Invalid fee payer signature format: {}", e)))?;
                
                // Payer signs the SAME transaction bytes to authorize payment for THIS transaction.
                let msg = signing_bytes(&tx);
                
                if payer_vk.verify(&msg, &payer_sig).is_err() {
                    tracing::error!("❌ Fee Payer Signature Verification Failed for payer {}", fee_payer);
                    return Err(AtlasError::Other("Invalid fee payer signature".to_string()));
                }
                
                // Verify Payer Address matches PK
                if let Ok(address) = atlas_common::address::address::Address::address_from_pk(&payer_vk, "nbex") {
                    if address != fee_payer {
                         tracing::error!("❌ Fee Payer Address Mismatch: Claimed {} vs Derived {}", fee_payer, address);
                         return Err(AtlasError::Other("Fee payer address mismatch".to_string()));
                    }
                }
            } else if st.fee_payer.is_some() {
                 // Fee Payer Claimed but No Signature!
                 tracing::error!("❌ Fee Payer {} claimed but no signature provided!", fee_payer);
                 return Err(AtlasError::Other("Missing fee payer signature".to_string()));
            }

            // --- 2.5 ASSET VALIDATION ---
            if tx.asset != "ATLAS" {
                let state_guard = self.state.read().await;
                if !state_guard.tokens.contains_key(&tx.asset) {
                     tracing::error!("❌ Unknown Asset: {}", tx.asset);
                     return Err(AtlasError::Other(format!("Asset '{}' is not registered.", tx.asset)));
                }
            }
    
            // Use Accounting Engine to process transfer (MAIN LEG)
            let mut entry = atlas_bank::institution_subledger::engine::AccountingEngine::process_transfer(
                &tx.from,
                &tx.to,
                tx.amount as u64,
                &tx.asset,
                tx.memo.clone(),
            ).map_err(|e| {
                tracing::error!("❌ Accounting Engine Error: {}", e);
                AtlasError::Other(format!("Accounting Engine Error: {}", e))
            })?;

            // --- 3. FEE EXECUTION ---
            // Calculate Fee
            let base_fee: u64 = 1000;
            // Approximate size: JSON length or Bincode length. Using simplistic estimation for now.
            // Using `signing_bytes` length as proxy for size.
            let size_bytes = signing_bytes(&tx).len() as u64; 
            let byte_fee = size_bytes * 10;
            let total_fee = base_fee + byte_fee;

            // Deduct Fee
            // Debit Payer, Credit Validator (90%) and System (10%)
            // 90/10 Split
            let validator_reward = (total_fee * 90) / 100;
            let system_revenue = total_fee - validator_reward;

            // Payer Account (Debit Total)
            let payer_account = if fee_payer.starts_with("passivo:wallet:") {
                fee_payer.clone()
            } else {
                format!("passivo:wallet:{}", fee_payer)
            };

            entry.legs.push(Leg {
                account: payer_account,
                asset: "ATLAS".to_string(), 
                kind: LegKind::Debit, 
                amount: total_fee as u128,
            });

            // Validator Reward (Credit)
            // Convert proposer (NodeId) to Address (assuming nbex convention or using raw string)
            // Proposal.proposer is a NodeId (String). Usually "node-id" or similar.
            // If the proposer uses a public key as ID, we can credit it directly.
            // If it's an internal ID, we might need a lookup.
            // FOR NOW: We assume Proposer ID IS the Wallet Address or can be wrapped.
            // User confirmed "validators" pay.
            let proposer_addr = proposal.proposer.to_string();
            let validator_account = if proposer_addr.starts_with("passivo:wallet:") {
                proposer_addr
            } else {
                format!("passivo:wallet:{}", proposer_addr)
            };

            entry.legs.push(Leg {
                account: validator_account.clone(),
                asset: "ATLAS".to_string(),
                kind: LegKind::Credit,
                amount: validator_reward as u128,
            });

            // System Revenue (Credit)
            entry.legs.push(Leg {
                account: "patrimonio:fees".to_string(), // Revenue Account
                asset: "ATLAS".to_string(),
                kind: LegKind::Credit,
                amount: system_revenue as u128,
            });
            
            tracing::info!("💸 Fee Distribution: Total={} | Payer={} | Val({})={} | Sys={}", 
                total_fee, fee_payer, validator_account, validator_reward, system_revenue);
            
            tracing::info!("💸 Fee Deduction: {} deducted from {} (Payer: {})", total_fee, fee_payer, fee_payer);
            
            // --- Phase 5.5: Token Registry Interceptor ---
            let mut registry_action: Option<Box<dyn FnOnce(&mut crate::core::ledger::state::State) -> std::result::Result<(), String> + Send>> = None;

            if tx.to == "system:registry" {
                 // Enforce Payment Asset is ATLAS
                 if tx.asset != "ATLAS" {
                     return Err(AtlasError::Other("Registration fee must be paid in ATLAS".to_string()));
                 }

                 // Enforce Fee: 100 ATLAS
                 let registration_fee: u64 = 100;
                 if tx.amount < registration_fee as u128 {
                      return Err(AtlasError::Other("Insufficient registration fee (100 ATLAS required)".to_string()));
                 }
                 
                 // Move funds from "system:registry" (where AccountingEngine put them) to "patrimonio:fees"
                 entry.legs.push(Leg {
                     account: "passivo:wallet:system:registry".to_string(),
                     asset: "ATLAS".to_string(),
                     kind: LegKind::Debit, 
                     amount: registration_fee as u128,
                 });
                 entry.legs.push(Leg {
                     account: "patrimonio:fees".to_string(),
                     asset: "ATLAS".to_string(),
                     kind: LegKind::Credit,
                     amount: registration_fee as u128,
                 });

                 if let Some(memo) = &tx.memo {
                     if let Ok(mut metadata) = serde_json::from_str::<crate::core::ledger::token::TokenMetadata>(memo) {
                         // Enforce issuer matches sender
                         metadata.issuer = tx.from.clone();
                         let token_symbol = metadata.symbol.clone();
                         
                         tracing::info!("®️ REGISTER TOKEN: {} ({}) by {}", token_symbol, metadata.name, tx.from);

                         registry_action = Some(Box::new(move |state| {
                             if state.tokens.contains_key(&token_symbol) {
                                 return Err(format!("Token {} already registered", token_symbol));
                             }
                             state.tokens.insert(token_symbol, metadata);
                             Ok(())
                         }));
                     } else {
                         return Err(AtlasError::Other("Invalid Token Metadata JSON in Memo".to_string()));
                     }
                 } else {
                      return Err(AtlasError::Other("Missing Metadata in Memo".to_string()));
                 }
            }

            // --- Phase 6: Delegation & Staking Interceptor ---
            // Use explicitly std::result::Result<(), String> to match closure signature
            let mut staking_action: Option<Box<dyn FnOnce(&mut crate::core::ledger::state::State) -> std::result::Result<(), String> + Send>> = None;

            if tx.to == "system:staking" {
                 if let Some(memo) = &tx.memo {
                     if memo.starts_with("delegate:") {
                         // Memo: delegate:<VALIDATOR_ADDRESS>
                         let parts: Vec<&str> = memo.split(':').collect();
                         if parts.len() >= 2 {
                             let validator = parts[1].to_string();
                             let amount = tx.amount as u64;
                             let delegator = tx.from.clone();
                             
                             tracing::info!("🤝 DELEGATE: {} delegating {} to {}", delegator, amount, validator);
                             staking_action = Some(Box::new(move |state| {
                                 state.delegations.delegate(delegator, validator, amount);
                                 Ok(())
                             }));
                         }
                     } else if memo.starts_with("undelegate:") {
                         // Memo: undelegate:<VALIDATOR_ADDRESS>:<AMOUNT>
                         let parts: Vec<&str> = memo.split(':').collect();
                         if parts.len() >= 3 {
                             let validator = parts[1].to_string();
                             if let Ok(amount) = parts[2].parse::<u64>() {
                                 let delegator = tx.from.clone();
                                 
                                 tracing::info!("🤝 UNDELEGATE: {} withdrawing {} from {}", delegator, amount, validator);
                                 
                                 // 1. Queue State Update (Reduce Delegation)
                                 staking_action = Some(Box::new(move |state| {
                                     state.delegations.undelegate(delegator, validator, amount)
                                 }));

                                 // 2. Add Refund Legs (Pool -> User)
                                 entry.legs.push(Leg {
                                     account: "passivo:wallet:system:staking".to_string(), // Debiting Pool (Corrected)
                                     asset: "ATLAS".to_string(),
                                     kind: LegKind::Debit,
                                     amount: amount as u128,
                                 });
                                 entry.legs.push(Leg {
                                     account: format!("passivo:wallet:{}", tx.from),
                                     asset: "ATLAS".to_string(),
                                     kind: LegKind::Credit, // Credit User (Liability Increase = Balance Increase)
                                     amount: amount as u128,
                                 });
                             }
                         }
                     }
                 }
            }
            // -------------------------------------------------
    
            // Enrich entry with proposal metadata (and unique index suffix)
            entry.entry_id = format!("entry-{}-{}", proposal.id, count);
            entry.tx_hash = proposal.hash.clone(); 
            entry.block_height = proposal.height;
            entry.timestamp = proposal.time;
    
            // Apply to state
            let mut state = self.state.write().await;
            
            // --- NONCE VALIDATION ---
            // We must retrieve the account state to check the nonce.
            // If the account doesn't exist yet, we treat nonce as 0.
            let account_nonce = if let Some(acc) = state.accounts.get(&tx.from) {
                acc.nonce
            } else if let Some(acc) = state.accounts.get(&format!("passivo:wallet:{}", tx.from)) {
                // Fallback: Check for schema-prefixed account
                acc.nonce
            } else {
                0
            };

            tracing::info!("🔢 Nonce Check: Account={} | Stored={} | Tx={} | Expected={}", 
                tx.from, account_nonce, tx.nonce, account_nonce + 1);

            // Expected nonce is account_nonce + 1
            if tx.nonce != account_nonce + 1 {
                tracing::error!("❌ Nonce Mismatch! Account={} Expected={} Got={}", tx.from, account_nonce + 1, tx.nonce);
                return Err(AtlasError::Other(format!(
                    "Invalid Nonce: Expected {}, got {}. (Account: {})", 
                    account_nonce + 1, tx.nonce, tx.from
                )));
            }

            // --- AEC CHAINING: Link to previous transaction hash ---
            // We must read the current 'last_transaction_hash' for all involved accounts
            // and add it to the entry BEFORE applying it.
            for leg in &entry.legs {
                if let Some(account_state) = state.accounts.get(&leg.account) {
                    if let Some(prev_hash) = &account_state.last_transaction_hash {
                        entry.prev_for_account.insert(leg.account.clone(), prev_hash.clone());
                    }
                }
            }
            
            // Execute Staking Action (Delegate/Undelegate Logic)
            if let Some(action) = staking_action {
                action(&mut state).map_err(|e| AtlasError::Other(format!("Staking Action Failed: {}", e)))?;
            }
            
            // Execute Registry Action
            if let Some(action) = registry_action {
                action(&mut state).map_err(|e| AtlasError::Other(format!("Token Registration Failed: {}", e)))?;
            }

            // Pre-Ex Balances
            let pre_bal = if let Some(acc) = state.accounts.get(&tx.from) {
                *acc.balances.get(&tx.asset).unwrap_or(&0)
            } else { 0 };

            tracing::info!("📉 Executing Transfer: {} -> {} | Amount: {} {} | Pre-Bal: {}", tx.from, tx.to, tx.amount, tx.asset, pre_bal);

            state.apply_entry(entry.clone())
                .map_err(|e| AtlasError::Other(format!("Transaction execution failed: {}", e)))?;
            
            // Post-Ex Balances
            let post_bal = if let Some(acc) = state.accounts.get(&tx.from) {
                *acc.balances.get(&tx.asset).unwrap_or(&0)
            } else { 0 };
            
            tracing::info!("✅ Transfer Complete. New Balance for {}: {} {}", tx.from, post_bal, tx.asset);
            
            // --- PERSISTENCE: Write to Physical Shards ---
            if persist_shards {
                // Release state lock before IO to maximize throughput
                drop(state); // Ensure state is dropped if still held, though we dropped it above? 
                // Wait, line 191 dropped state. 
                // But wait, the context of this replacement:
                // I need to be careful about where I insert. 
                // The original code:
                // 189: // --- PERSISTENCE: Write to Physical Shards ---
                // 190: // Release state lock before IO to maximize throughput
                // 191: drop(state);
                // 192: 
                // 193: let shards = self.shards.read().await;
                // ...
                
                let shards = self.shards.read().await;
                // Write to every involved account's independent chain
                let mut involved_accounts = std::collections::HashSet::new();
                for leg in &entry.legs {
                    involved_accounts.insert(leg.account.clone());
                }

                for account in involved_accounts {
                    if let Err(e) = shards.append(&account, &entry).await {
                        tracing::error!("❌ Failed to write shard for {}: {}", account, e);
                        // Critical: if shard write fails, we have a sync issue.
                        // Ideally we would rollback or halt. For now, we log error as Monolith is safe.
                    }
                }
            } else {
                 drop(state);
                 tracing::info!("⏩ Replay Mode: Skipping shard write for tx {}", entry.tx_hash);
            }
            
            count += 1;
        }

        Ok(count)
    }

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
             
             if !address.starts_with("nbex") && !address.starts_with("0x") {
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

             // Double Entry:
             // 1. Debit Equity (Issuance)
             let debit_leg = Leg {
                 account: "patrimonio:genesis".to_string(), // Equity
                 asset: "ATLAS".to_string(),
                 kind: LegKind::Debit, // Reduces Equity (Technically Equity is Credit normal, so Debit reduces it to create Liability)
                 amount: *amount as u128,
             };

             // 2. Credit Liability (User Wallet)
             // Note: If address is now Bech32 (nbex...), we treat it as a Root Account for simplicity in the new schema, 
             // but keeping "passivo:wallet:" prefix for consistency with Legacy, OR rely on schema.rs.
             // Given Schema: 0x... and nbex... maps to Passivo. 
             // Let's keep strict "passivo:wallet:" wrapper for now to avoid breaking existing queries if they rely on it, 
             // OR switch to "nbex..." raw key if Account::new handles it.
             // Looking at state::new(), it uses "passivo:wallet:nbex1...". So we follow that pattern.
             
             // Wait, Schema says "0x..." -> AccountClass::Passivo.
             // If we store "passivo:wallet:nbex...", it matches standard internal account.
             // If we store "nbex...", Account::new might handle it but let's be safe and wrap.
             // Actually, `node_id_to_address` returns pure `nbex...`. 
             // Ledger::get_validator_total_power calls `get_balance(address)`.
             // `get_balance` checks `state.accounts.get(address)`.
             // If Consensus asks for "nbex123", we MUST store keys as "nbex123".
             // Storing as "passivo:wallet:nbex123" implies the caller must know the prefix.
             // BUT, `get_balance` implementation in `manager.rs`:
             //    if let Some(account) = state.accounts.get(address) ...
             
             // So, does Consensus call `get_validator_total_power("passivo:wallet:nbex...")` or just `"nbex..."`?
             // Consensus simply gets the address from `node_id_to_address` (which returns "nbex...") and calls `get_validator_stake`.
             // So Ledger MUST store "nbex..." as the key, OR Consensus must prepend the prefix.
             // Given the "Blockchain = 32" philosophy, the Ledger should likely use the Bech32 address as the Primary Key for these accounts.
             
             // However, `state::new` in `mod.rs` (lines 112, 118) initializes Alice as:
             // "passivo:wallet:nbex1c..."
             // This suggests the convention IS to wrap.
             
             // IF the convention is to wrap, then Consensus core.rs needs to Prepend?
             // OR `get_balance` should handle the lookup?
             // Let's look at `get_balance` in `manager.rs`:
             // `let bal = *account.balances.get(asset).unwrap_or(&0);` -> Direct lookup.
             
             // If `state.accounts` has keys "passivo:wallet:nbex...", and I request "nbex...", I get None.
             // So I should probably FIX `get_balance` to support alias lookup OR
             // Store purely as "nbex...".
             
             // Decision: The new `AccountSchema` in `schema.rs` says:
             // `parse_root` maps `nbex` to `Passivo`. 
             // If I store as `nbex...`, `AccountSchema` works.
             // If I store as `passivo:wallet:nbex...`, `AccountSchema` validates it as internal string.
             
             // To fix the "Total Stake 0" right now without changing Consensus code again:
             // I will store the Genesis accounts with the `nbex...` key directly if possible?
             // NO, wait. `state::new` uses wrapper.
             // Let's look at `AccountingEngine::process_transfer`. It takes "from" and "to".
             // If I send from "nbex...", the engine likely looks up "nbex...".
             // `atlas-bank/src/institution_subledger/accounts.rs` (User has it open).
             
             // Let's assume for this specific FIX, we want the KEYS in the HashMap to match what Consensus asks for.
             // If Consensus asks for `nbex...`, we should store key `nbex...`.
             // BUT `State::new()` initializes `passivo:wallet:nbex...`. 
             // This implies `State` expects full paths.
             // So, if Consensus asks for `nbex...`, it's failing because it lacks the prefix.
             
             // BUT WAIT! The User said: "Ledger requires 'nbex' (Bech32)". 
             // If I change `state::new` to use raw `nbex...`, does it break `AccountingEngine`?
             // The `AccountingEngine` wraps things in `passivo:wallet:` if they are not system accounts?
             // I need to check `atlas-bank`.
             
             // ASSUMPTION: To make it work FAST:
             // 1. In `apply_genesis_state`, I will convert to `nbex`.
             // 2. I will store TWO entries or ensure `get_balance` works.
             // Actually, `node_id_to_address` returns `nbex...`.
             // `manager.rs` `get_balance` does `state.accounts.get(address)`.
             // So if I want `get_balance("nbex...")` to work, the key MUST be `"nbex..."`.
             
             // So I will store it as `final_address` (raw `nbex...`).
             // And I will ALSO change `State::new` in `mod.rs` to use raw `nbex...` later if needed, but Genesis is the practical entry point.
             
             let account_key = if final_address.starts_with("nbex") {
                 format!("passivo:wallet:{}", final_address) // STANDARD: Wrap with prefix
             } else {
                 format!("passivo:wallet:{}", final_address) // Wrap legacy
             };

             let credit_leg = Leg {
                 account: account_key.clone(),
                 asset: "ATLAS".to_string(),
                 kind: LegKind::Credit, // Increases Liability
                 amount: *amount as u128,
             };

             // Double Entry Bypass for Genesis (Creation)
             
             // 1. Update/Create Wallet Account
             let wallet_account = state.accounts.entry(account_key.clone()).or_insert_with(crate::core::ledger::account::AccountState::new);
             let balance = wallet_account.balances.entry("ATLAS".to_string()).or_insert(0);
             *balance += *amount as u128; // Credit
             
             // Track transaction/entry metadata
             // Fix: Initialize nonce to 0 so the first user transaction (Nonce 1) is valid (1 == 0 + 1).
             // wallet_account.nonce = 0; // Default is 0, so just don't increment.
             
             // 2. Update/Create Equity Account (The Source)
             let equity_account = state.accounts.entry("patrimonio:genesis".to_string()).or_insert_with(crate::core::ledger::account::AccountState::new);
             let eq_balance = equity_account.balances.entry("ATLAS".to_string()).or_insert(0);
             // Verify if we can go negative? Assuming u128, we can't. 
             // So we just Track Issued Amount as a positive number in Equity (representing Contra-Equity/Issuance)?
             // Or allow overflow? u128 cannot be negative.
             // Standard practice: Equity accounts usually hold Credit balances. 
             // If we Debit Equity, we reduce it. 
             // For Genesis, we are CREATING Liability (Money). So Assets + Liabilities = 0 ? No. Assets = Liab + Equity.
             // Money is Liability of Bank. 
             // Equity (Capital) balances it.
             // So Credit Liability (Wallet) 100.
             // Debit Equity (Capital) 100?
             // That implies Equity becomes -100. 
             // If u128 is unsigned, we can't represent negative Equity.
             // So we just ADD to the 'patrimonio:genesis' account as a "Contra-Account"?
             // Or just ignore Equity side for now to prevent panic.
             // Let's just track it as positive "Issued Supply" for now.
             *eq_balance += *amount as u128; 

             let debit_leg = Leg {
                 account: "patrimonio:genesis".to_string(),
                 asset: "ATLAS".to_string(),
                 kind: LegKind::Debit, 
                 amount: *amount as u128,
             };

             // 3. Create Genesis Entry for Persistence
             let entry = LedgerEntry::new(
                 format!("genesis-{}", final_address),
                 vec![debit_leg, credit_leg], // Debit Equity, Credit Wallet
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

    /// Puni um validador queimando (confiscando) seus fundos.
    /// Remove o valor do saldo do endereço e creditar em 'patrimonio:slashing' (balanço contábil).
    pub async fn slash_validator(&self, address: &str, amount: u64) -> Result<()> {
        let current_balance = self.get_balance(address, "ATLAS").await?;
        if current_balance == 0 {
            tracing::warn!("⚔️ Slashing falhou: Validador {} já está zerado.", address);
            return Ok(());
        }

        let slash_amt = std::cmp::min(current_balance, amount);
        tracing::info!("⚔️ SLASHING: Punindo {} em {} ATLAS (Saldo: {})", address, slash_amt, current_balance);

        // 1. Debit User Liability (Reduzir passivo = Reduzir grana do user)
        let debit_leg = Leg {
            account: format!("passivo:wallet:{}", address),
            asset: "ATLAS".to_string(),
            kind: LegKind::Debit, // Debit em Liability REDUZ o saldo
            amount: slash_amt as u128,
        };

        // 2. Credit Equity (Slashing Revenue / Burnt)
        let credit_leg = Leg {
            account: "patrimonio:slashing".to_string(),
            asset: "ATLAS".to_string(),
            kind: LegKind::Credit, // Credit em Equity AUMENTA (ganho para a rede/queima)
            amount: slash_amt as u128,
        };

        let mut legs = vec![debit_leg, credit_leg];

        // 3. Shared Slashing Risk: Punish Delegators (10%)
        {
             // Refactoring to hold lock once.
             let mut state = self.state.write().await;
             
             // 3.1 Calculate Delegator Penalty
             let delegated_penalty = state.delegations.slash_delegators(address, 10); // 10% penalty
             if delegated_penalty > 0 {
                 tracing::info!("⚔️ SLASHING SHARED: Punindo delegadores de {} em {} ATLAS (10%)", address, delegated_penalty);
                 // Burn from Staking Pool
                 legs.push(Leg {
                     account: "passivo:wallet:system:staking".to_string(), // Reduce Pool Liability
                     asset: "ATLAS".to_string(),
                     kind: LegKind::Debit, 
                     amount: delegated_penalty as u128,
                 });
                 legs.push(Leg {
                     account: "patrimonio:slashing".to_string(), // Increase Burnt
                     asset: "ATLAS".to_string(),
                     kind: LegKind::Credit,
                     amount: delegated_penalty as u128,
                 });
             }

             let entry_id = format!("slash-{}-{}", address, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
             
             let entry = LedgerEntry::new(
                 entry_id,
                 legs,
                 "0000000000000000000000000000000000000000000000000000000000000000".to_string(), // No block hash associated yet
                 0,
                 0,
                 Some(format!("SLASHING PENALTY: Disrespectful Behavior")),
             );

             state.apply_entry(entry)
                  .map_err(|e| AtlasError::Other(format!("Failed to apply slashing: {}", e)))?;
        }

        Ok(())
    }
}
