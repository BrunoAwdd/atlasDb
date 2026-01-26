use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use atlas_common::error::Result;
use crate::core::runtime::{binlog::Binlog, index::Index};
use crate::core::ledger::state::State;
use crate::core::ledger::storage::shards::ShardStorage;

#[derive(Debug)]
pub struct Ledger {
    pub binlog: Arc<RwLock<Binlog>>,
    pub index: Arc<RwLock<Index>>,
    pub state: Arc<RwLock<State>>,
    pub shards: Arc<RwLock<ShardStorage>>,
}

impl Ledger {
    pub async fn new(data_dir: &str) -> Result<Self> {
        let binlog = Binlog::new(data_dir).await?;
        let index = Index::new(data_dir)?;
        let shards = ShardStorage::new(data_dir).await?;
        
        let ledger = Self {
            binlog: Arc::new(RwLock::new(binlog)),
            index: Arc::new(RwLock::new(index)),
            state: Arc::new(RwLock::new(State::new())),
            shards: Arc::new(RwLock::new(shards)),
        };

        // Replay Binlog to restore State
        let proposals = ledger.get_all_proposals().await?;
        if !proposals.is_empty() {
            println!("Replaying {} transactions from WAL...", proposals.len());
            for proposal in proposals {
                if let Err(e) = ledger.execute_transaction(&proposal, true).await {
                    eprintln!("Failed to replay transaction {}: {}", proposal.id, e);
                    // Decide if we panic or continue. Warn for now.
                }
            }
        }

        Ok(ledger)
    }

    pub async fn get_balance(&self, address: &str, asset: &str) -> Result<u64> {
        let state = self.state.read().await;
        
        // 1. Try Direct Lookup
        if let Some(account) = state.accounts.get(address) {
            let bal = *account.balances.get(asset).unwrap_or(&0);
            return Ok(bal.try_into().unwrap_or(u64::MAX));
        }

        // 2. Try Prefix Lookup (Legacy/Schema Compat)
        if !address.contains(':') {
            let prefixed = format!("passivo:wallet:{}", address);
            if let Some(account) = state.accounts.get(&prefixed) {
                let bal = *account.balances.get(asset).unwrap_or(&0);
                return Ok(bal.try_into().unwrap_or(u64::MAX));
            }
        }

        Ok(0)
    }

    pub async fn exists_proposal(&self, id: &str) -> bool {
        let index = self.index.read().await;
        // Check if we can locate it in the index
        matches!(index.get_proposal_location(id), Ok(Some(_)))
    }

    /// Validates a proposal deeply before persistence (Signature + Content + Nonce).
    /// Used for "Last Mile Protection" to ensure no garbage enters the Binlog.
    pub async fn validate_proposal_hard(&self, proposal: &atlas_common::env::proposal::Proposal) -> Result<()> {
        use atlas_common::env::proposal::signing_bytes;
        use ed25519_dalek::{Verifier, Signature, VerifyingKey};

        // 1. Verify Proposal Signature
        let public_key_bytes: [u8; 32] = proposal.public_key.clone().try_into()
            .map_err(|_| atlas_common::error::AtlasError::Auth("Invalid proposal public key len".to_string()))?;
        
        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
             .map_err(|e| atlas_common::error::AtlasError::Auth(format!("Invalid proposal public key: {}", e)))?;
        
        let signature = Signature::from_slice(&proposal.signature)
             .map_err(|e| atlas_common::error::AtlasError::Auth(format!("Invalid proposal signature fmt: {}", e)))?;
        
        // Reconstruct signing bytes
        let sign_msg = signing_bytes(proposal);
        
        verifying_key.verify(&sign_msg, &signature)
             .map_err(|e| atlas_common::error::AtlasError::Auth(format!("Proposal signature verification failed: {}", e)))?;

        // 2. Parse and Verify Transactions (Signatures + Nonce)
        let transactions: Vec<atlas_common::transactions::SignedTransaction> = 
            if let Ok(batch) = serde_json::from_str::<Vec<atlas_common::transactions::SignedTransaction>>(&proposal.content) {
                batch
            } else if let Ok(signed_tx) = serde_json::from_str::<atlas_common::transactions::SignedTransaction>(&proposal.content) {
                vec![signed_tx]
            } else {
                 return Err(atlas_common::error::AtlasError::Other("Invalid proposal content format".to_string()));
            };

        let state = self.state.read().await;

        for (i, tx) in transactions.iter().enumerate() {
            // A. Stateless Validation (Signature)
            tx.validate_stateless()
                .map_err(|e| atlas_common::error::AtlasError::Auth(format!("Tx {} invalid signature: {}", i, e)))?;

            // B. Stateful Validation (Nonce)
            // We check if Nonce == CurrentNonce + 1.
            // Note: In a block with multiple txs from same sender, nonces must be sequential.
            // But usually blocks are batched. For simplicity we check against State (strict) or allow "pending"?
            // Strict check: State Nonce is N. Tx Nonce must be N+1.
            // If multiple txs from same sender in same block: Tx1(N+1), Tx2(N+2)... this logic requires tracking "temp nonce" valid for this block.
            // For now, let's just check it's > State Nonce to prevent replay. Stricter check requires ordering context.
            // User asked for "nonce" check. Preventing replay (nonce <= state.nonce) is key.
            
            let sender = &tx.transaction.from;
            let current_nonce = if let Some(acct) = state.accounts.get(sender) {
                acct.nonce
            } else {
                0
            };

            if tx.transaction.nonce <= current_nonce {
                return Err(atlas_common::error::AtlasError::Storage(format!("Tx {} Replay detected! Nonce {} <= Current {}", i, tx.transaction.nonce, current_nonce)));
            }
        }

        Ok(())
    }

    pub async fn get_all_accounts(&self) -> HashMap<String, crate::core::ledger::account::AccountState> {
        let state = self.state.read().await;
        state.accounts.clone()
    }

    pub async fn get_all_tokens(&self) -> HashMap<String, crate::core::ledger::token::TokenMetadata> {
        let state = self.state.read().await;
        state.tokens.clone()
    }
}
