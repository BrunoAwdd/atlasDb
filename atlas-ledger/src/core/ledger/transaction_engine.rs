use atlas_common::{
    env::proposal::Proposal,
    error::{Result, AtlasError},
    transactions::{Transaction, SignedTransaction, signing_bytes},
    entry::{Leg, LegKind, LedgerEntry},
    genesis::GenesisState
};
use ed25519_dalek::{Verifier, VerifyingKey, Signature};
use crate::Ledger;

impl Ledger {
    /// Returns the total voting power of a validator (Own Stake + Delegations).
    pub async fn get_validator_total_power(&self, address: &str) -> Result<u64> {
        let own_balance = self.get_balance(address, "ATLAS").await?;
        let state = self.state.read().await;
        let delegated_power = state.delegations.get_delegated_power(address);
        Ok(own_balance + delegated_power)
    }

    /// Executes a transaction batch (proposal content) and updates the state.
    /// Returns the number of executed transactions.
    pub async fn execute_transaction(&self, proposal: &Proposal) -> Result<usize> {
        // 1. Try Batch Parsing
        let transactions: Vec<(Transaction, Option<Vec<u8>>, Option<Vec<u8>>)> = 
            if let Ok(batch) = serde_json::from_str::<Vec<SignedTransaction>>(&proposal.content) {
                batch.into_iter().map(|st| (st.transaction, Some(st.signature), Some(st.public_key))).collect()
            } else if let Ok(signed_tx) = serde_json::from_str::<SignedTransaction>(&proposal.content) {
                // Fallback: Single SignedTransaction
                vec![(signed_tx.transaction, Some(signed_tx.signature), Some(signed_tx.public_key))]
            } else {
                // Fallback: Legacy Single Transaction (Unsigned)
                let tx: Transaction = serde_json::from_str(&proposal.content)
                    .map_err(|e| AtlasError::Other(format!("Failed to parse transaction content: {}", e)))?;
                vec![(tx, None, None)]
            };

        let mut count = 0;
        for (tx, signature, public_key) in transactions {
            // If signed, verify signature
            if let (Some(sig), Some(pk)) = (signature, public_key) {
                 let verifying_key = VerifyingKey::from_bytes(pk.as_slice().try_into().unwrap_or(&[0u8; 32]))
                    .map_err(|e| AtlasError::Other(format!("Invalid public key: {}", e)))?;
                 let signature = Signature::from_slice(&sig)
                    .map_err(|e| AtlasError::Other(format!("Invalid signature format: {}", e)))?;
                 let msg = signing_bytes(&tx);
    
                 if verifying_key.verify(&msg, &signature).is_err() {
                     return Err(AtlasError::Other("Invalid transaction signature".to_string()));
                 }
            } else {
                 println!("⚠️ Executing unsigned transaction (legacy path)");
            }
    
            // Use Accounting Engine to process transfer
            let mut entry = atlas_bank::institution_subledger::engine::AccountingEngine::process_transfer(
                &tx.from,
                &tx.to,
                tx.amount as u64,
                &tx.asset,
                tx.memo.clone(),
            ).map_err(|e| AtlasError::Other(format!("Accounting Engine Error: {}", e)))?;
            
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
            
            // Execute Staking Action (Delegate/Undelegate Logic)
            if let Some(action) = staking_action {
                action(&mut state).map_err(|e| AtlasError::Other(format!("Staking Action Failed: {}", e)))?;
            }

            state.apply_entry(entry.clone())
                .map_err(|e| AtlasError::Other(format!("Transaction execution failed: {}", e)))?;
            
            count += 1;
        }

        Ok(count)
    }

    /// Applies the genesis state if the ledger is empty.
    pub async fn apply_genesis_state(&self, genesis: &GenesisState) -> Result<()> {
        let mut state = self.state.write().await;
        
        // Simple check: if we have any accounts, genesis was already applied
        if !state.accounts.is_empty() {
            // Check if we strictly want to skip or verify. 
        }
        
        for (address, amount) in &genesis.allocations {
             // Double Entry:
             // 1. Debit Equity (Issuance)
             let debit_leg = Leg {
                 account: "patrimonio:genesis".to_string(), // Equity
                 asset: "ATLAS".to_string(),
                 kind: LegKind::Debit, // Reduces Equity (Technically Equity is Credit normal, so Debit reduces it to create Liability)
                 amount: *amount as u128,
             };

             // 2. Credit Liability (User Wallet)
             let credit_leg = Leg {
                 account: format!("passivo:wallet:{}", address), // Liability
                 asset: "ATLAS".to_string(),
                 kind: LegKind::Credit, // Increases Liability
                 amount: *amount as u128,
             };

             let entry_id = format!("genesis-{}", address);
             let entry = LedgerEntry::new(
                 entry_id,
                 vec![debit_leg, credit_leg],
                 "0000000000000000000000000000000000000000000000000000000000000000".to_string(), // Genesis Hash
                 0,
                 0,
                 Some("Genesis Allocation".to_string()),
             );

             tracing::info!("🏛️ Applying Genesis: {} -> {} ATLAS", address, amount);
             state.apply_entry(entry)
                .map_err(|e| AtlasError::Other(format!("Failed to apply genesis: {}", e)))?;
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
