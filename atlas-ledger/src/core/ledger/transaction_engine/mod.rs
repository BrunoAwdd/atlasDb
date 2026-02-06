use atlas_common::{
    env::proposal::Proposal,
    error::{Result, AtlasError},
    transactions::{Transaction, SignedTransaction},
    entry::{LedgerEntry},
};
use crate::Ledger;

pub mod validation;
pub mod fees;
pub mod inflation;
pub mod interceptors;

use validation::ValidationHandler;
use fees::FeeHandler;
use inflation::InflationHandler;
use interceptors::{InterceptorHandler, InterceptorAction};

impl Ledger {
    /// Returns the total voting power of a validator.
    pub async fn get_validator_total_power(&self, address: &str) -> Result<u64> {
        // Use default full ID for staking power
        let own_balance = self.get_balance(address, crate::core::ledger::asset::ATLAS_FULL_ID).await?;
        let state = self.state.read().await;
        let delegated_power = state.delegations.get_delegated_power(address);
        
        tracing::info!("🔍 Stake Query: Addr={} | Balance={} | Delegated={} | Total={}", 
            address, own_balance, delegated_power, own_balance + delegated_power);
            
        Ok(own_balance + delegated_power)
    }

    /// Executes a transaction batch (proposal content) and updates the state.
    /// Orchestrates the 7-step pipeline.
    pub async fn execute_transaction(&self, proposal: &Proposal, persist_shards: bool) -> Result<usize> {
        // 1. Parsing
        let transactions = self.parse_transactions(proposal)?;
        
        let mut count = 0;
        let proposer_id = proposal.proposer.to_string();

        for (idx, st) in transactions.iter().enumerate() {
            let tx = &st.transaction;

            // 2. Stateless Validation
            if let Err(e) = self.validate_stateless(st, idx).await {
                return Err(e);
            }

            // 3. Accounting Engine (Main Transfer)
            let mut entry = self.create_base_entry(tx)?;

            // 4. Logic Pipeline (Enrich Entry checks & legs)
            // Returns Deferred Actions for State Mutation
            let actions = self.apply_business_logic(&mut entry, st, &proposal.public_key, &proposer_id)?;

            // 5. Metadata Enrichment
            self.enrich_metadata(&mut entry, proposal, count);

            // 6. State Mutation (Critical Section)
            self.commit_to_state(&mut entry, tx, actions).await?;

            // 7. Persistence (I/O)
            self.persist_to_storage(&entry, persist_shards).await;
            
            count += 1;
        }

        Ok(count)
    }

    // --- Helper Functions (SOLID Steps) ---

    fn parse_transactions(&self, proposal: &Proposal) -> Result<Vec<SignedTransaction>> {
        if let Ok(batch) = serde_json::from_str::<Vec<SignedTransaction>>(&proposal.content) {
            Ok(batch)
        } else if let Ok(signed_tx) = serde_json::from_str::<SignedTransaction>(&proposal.content) {
            Ok(vec![signed_tx])
        } else {
            // Fallback: Legacy
            let tx: Transaction = serde_json::from_str(&proposal.content)
                .map_err(|e| AtlasError::Other(format!("Failed to parse transaction content: {}", e)))?;
             Ok(vec![SignedTransaction {
                transaction: tx,
                signature: vec![],
                public_key: vec![],
                fee_payer: None,
                fee_payer_signature: None,
                fee_payer_pk: None,
            }])
        }
    }

    async fn validate_stateless(&self, st: &SignedTransaction, idx: usize) -> Result<()> {
        ValidationHandler::validate_signatures(st)
            .map_err(|e| {
                tracing::error!("❌ Validation Failed for Tx #{}: {}", idx, e);
                e
            })?;
            
        ValidationHandler::validate_asset(self, &st.transaction.asset).await?;
        Ok(())
    }

    fn create_base_entry(&self, tx: &Transaction) -> Result<LedgerEntry> {
        let asset_id = if tx.asset == crate::core::ledger::asset::ATLAS_SYMBOL {
            crate::core::ledger::asset::ATLAS_FULL_ID.to_string()
        } else {
            tx.asset.clone()
        };

        // Normalize addresses: Handle Raw, Wallet, and Legacy Passivo
        let sanitize = |addr: &str| -> String {
            if addr.starts_with("wallet:") {
                addr.replace("wallet:", "wallet:")
            } else if addr.contains(':') {
                addr.to_string()
            } else {
                format!("wallet:{}", addr)
            }
        };

        let from_addr = sanitize(&tx.from);
        let to_addr = sanitize(&tx.to);

        atlas_bank::institution_subledger::engine::AccountingEngine::process_transfer(
            &from_addr,
            &to_addr,
            tx.amount as u64,
            &asset_id,
            tx.memo.clone(),
        ).map_err(|e| {
            tracing::error!("❌ Accounting Engine Error: {}", e);
            AtlasError::Other(format!("Accounting Engine Error: {}", e))
        })
    }
    fn apply_business_logic(
        &self, 
        entry: &mut LedgerEntry, 
        st: &SignedTransaction, 
        proposer_pk: &[u8], 
        proposer_id: &str
    ) -> Result<Vec<InterceptorAction>> {
        let tx = &st.transaction;

        // 4.1 Fees
        FeeHandler::apply_fees(entry, st, proposer_pk, proposer_id)?;

        // 4.2 Inflation
        InflationHandler::apply_inflation(entry, st, proposer_pk, proposer_id)?;

        // 4.3 Interceptors
        let mut actions = Vec::new();
        if let Some(act) = InterceptorHandler::handle_registry(entry, tx)? {
            actions.push(act);
        }
        if let Some(act) = InterceptorHandler::handle_staking(entry, tx)? {
            actions.push(act);
        }

        Ok(actions)
    }

    fn enrich_metadata(&self, entry: &mut LedgerEntry, proposal: &Proposal, count: usize) {
        entry.entry_id = format!("entry-{}-{}", proposal.id, count);
        entry.tx_hash = proposal.hash.clone(); 
        entry.block_height = proposal.height;
        entry.timestamp = proposal.time;
    }

    async fn commit_to_state(
        &self, 
        entry: &mut LedgerEntry, 
        tx: &Transaction,
        actions: Vec<InterceptorAction>
    ) -> Result<()> {
        let mut state = self.state.write().await;

        Self::validate_nonce_stateful(&state, tx)?;
        Self::link_aec_chain(&state, entry);
        Self::execute_actions(&mut state, actions)?;
        Self::apply_entry_logged(&mut state, entry.clone(), tx)?;
        Self::increment_sender_nonce(&mut state, tx)?;

        Ok(())
    }
    // --- State Helpers ---
    
    fn increment_sender_nonce(state: &mut crate::core::ledger::state::State, tx: &Transaction) -> Result<()> {
        if let Some(acc) = state.accounts.get_mut(&tx.from) {
            acc.nonce += 1;
        } else if let Some(acc) = state.accounts.get_mut(&format!("wallet:{}", tx.from)) {
            acc.nonce += 1;
        } else {
            // Should be impossible if apply_entry created it, unless apply_entry failed?
            // But apply_entry result was checked.
            // Possibly the account created uses a different key?
            tracing::warn!("⚠️ Failed to increment nonce: Sender {} account not found after apply!", tx.from);
        }
        Ok(())
    }

    fn validate_nonce_stateful(state: &crate::core::ledger::state::State, tx: &Transaction) -> Result<()> {
        let _nonce = if let Some(acc) = state.accounts.get(&tx.from) {
            acc.nonce
        } else if let Some(acc) = state.accounts.get(&format!("wallet:{}", tx.from)) {
            acc.nonce
        } else {
            0
        };
        
        tracing::info!("🔢 Nonce Check: Account={} | Stored={} | Tx={} | Expected={}", 
            tx.from, _nonce, tx.nonce, _nonce + 1);

        if tx.nonce != _nonce + 1 {
            tracing::error!("❌ Nonce Mismatch! Account={} Expected={} Got={}", tx.from, _nonce + 1, tx.nonce);
            return Err(AtlasError::Other(format!(
                "Invalid Nonce: Expected {}, got {}. (Account: {})", 
                _nonce + 1, tx.nonce, tx.from
            )));
        }
        Ok(())
    }

    fn link_aec_chain(state: &crate::core::ledger::state::State, entry: &mut LedgerEntry) {
        for leg in &entry.legs {
            if let Some(account_state) = state.accounts.get(&leg.account) {
                if let Some(prev_hash) = &account_state.last_transaction_hash {
                    entry.prev_for_account.insert(leg.account.clone(), prev_hash.clone());
                }
            }
        }
    }

    fn execute_actions(state: &mut crate::core::ledger::state::State, actions: Vec<InterceptorAction>) -> Result<()> {
        for action in actions {
            action(state).map_err(|e| AtlasError::Other(format!("Action Execution Failed: {}", e)))?;
        }
        Ok(())
    }

    fn apply_entry_logged(state: &mut crate::core::ledger::state::State, entry: LedgerEntry, tx: &Transaction) -> Result<()> {
        let pre_bal = if let Some(acc) = state.accounts.get(&tx.from) {
            *acc.balances.get(&tx.asset).unwrap_or(&0)
        } else { 0 };

        tracing::info!("📉 Executing Transfer: {} -> {} | Amount: {} {} | Pre-Bal: {}", tx.from, tx.to, tx.amount, tx.asset, pre_bal);

        state.apply_entry(entry)
            .map_err(|e| AtlasError::Other(format!("Transaction apply failed: {}", e)))?;
        
        let post_bal = if let Some(acc) = state.accounts.get(&tx.from) {
            *acc.balances.get(&tx.asset).unwrap_or(&0)
        } else { 0 };
        
        tracing::info!("✅ Transfer Complete. New Balance for {}: {} {}", tx.from, post_bal, tx.asset);
        Ok(())
    }

    async fn persist_to_storage(&self, entry: &LedgerEntry, persist: bool) {
        if persist {
            tracing::info!("💾 Persistence: Starting shard write for tx {}", entry.tx_hash);
            let shards = self.shards.read().await;
            
            let mut involved_accounts = std::collections::HashSet::new();
            for leg in &entry.legs {
                involved_accounts.insert(leg.account.clone());
            }

            for account in involved_accounts {
                if let Err(e) = shards.append(&account, &entry).await {
                    tracing::error!("❌ Failed to write shard for {}: {}", account, e);
                }
            }
        } else {
            tracing::info!("⏩ Replay Mode: Skipping shard write for tx {}", entry.tx_hash);
        }
    }
}
