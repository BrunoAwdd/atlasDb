// pub mod bank; // moved to atlas-bank
pub mod core;
pub mod interface;

use std::sync::Arc;
use tokio::sync::RwLock;
use atlas_common::env::proposal::Proposal;
use atlas_common::error::Result;

use crate::core::runtime::{binlog, index};
use crate::core::ledger::state;

#[derive(Debug)]
pub struct Ledger {
    binlog: Arc<RwLock<binlog::Binlog>>,
    index: Arc<RwLock<index::Index>>,
    pub state: Arc<RwLock<state::State>>,
}

impl Ledger {
    pub async fn new(data_dir: &str) -> Result<Self> {
        let binlog = binlog::Binlog::new(data_dir).await?;
        let index = index::Index::new(data_dir)?;
        
        let ledger = Self {
            binlog: Arc::new(RwLock::new(binlog)),
            index: Arc::new(RwLock::new(index)),
            state: Arc::new(RwLock::new(state::State::new())),
        };

        // Replay Binlog to restore State
        let proposals = ledger.get_all_proposals().await?;
        if !proposals.is_empty() {
            println!("Replaying {} transactions from WAL...", proposals.len());
            for proposal in proposals {
                if let Err(e) = ledger.execute_transaction(&proposal).await {
                    eprintln!("Failed to replay transaction {}: {}", proposal.id, e);
                    // Decide if we panic or continue. Warn for now.
                }
            }
        }

        Ok(ledger)
    }

    pub async fn append_proposal(&self, proposal: &Proposal) -> Result<()> {
        let mut binlog = self.binlog.write().await;
        let mut index = self.index.write().await;

        let (file_id, offset, len) = binlog.append(proposal).await?;
        index.index_proposal(&proposal.id, file_id, offset, len)?;
        
        Ok(())
    }
    
    pub async fn get_proposal(&self, id: &str) -> Result<Option<Proposal>> {
        let index = self.index.read().await;
        if let Some((file_id, offset, len)) = index.get_proposal_location(id)? {
            let binlog = self.binlog.read().await;
            return Ok(Some(binlog.read_proposal(file_id, offset, len).await?));
        }
        Ok(None)
    }

    pub async fn get_proposals_after(&self, height: u64) -> Result<Vec<Proposal>> {
        let binlog = self.binlog.read().await;
        let all = binlog.read_all().await?;
        Ok(all.into_iter().filter(|p| p.height > height).collect())
    }

    pub async fn get_all_proposals(&self) -> Result<Vec<Proposal>> {
        let binlog = self.binlog.read().await;
        binlog.read_all().await
    }

    /// Executes a transaction (proposal content) and updates the state.
    /// Returns the generated LedgerEntry.
    pub async fn execute_transaction(&self, proposal: &Proposal) -> Result<atlas_common::entry::LedgerEntry> {
        // Attempt to parse as SignedTransaction
        let (tx, signature, public_key) = if let Ok(signed_tx) = serde_json::from_str::<atlas_common::transaction::SignedTransaction>(&proposal.content) {
            (signed_tx.transaction, Some(signed_tx.signature), Some(signed_tx.public_key))
        } else {
            // Fallback for legacy
            let tx: atlas_common::transaction::Transaction = serde_json::from_str(&proposal.content)
                .map_err(|e| atlas_common::error::AtlasError::Other(format!("Failed to parse transaction: {}", e)))?;
            (tx, None, None)
        };

        // If signed, verify signature
        if let (Some(sig), Some(pk)) = (signature, public_key) {
             use ed25519_dalek::{Verifier, VerifyingKey, Signature};
             use atlas_common::transaction::signing_bytes;
             
             let verifying_key = VerifyingKey::from_bytes(pk.as_slice().try_into().unwrap_or(&[0u8; 32]))
                .map_err(|e| atlas_common::error::AtlasError::Other(format!("Invalid public key: {}", e)))?;
             let signature = Signature::from_slice(&sig)
                .map_err(|e| atlas_common::error::AtlasError::Other(format!("Invalid signature format: {}", e)))?;
             let msg = signing_bytes(&tx);

             if verifying_key.verify(&msg, &signature).is_err() {
                 return Err(atlas_common::error::AtlasError::Other("Invalid transaction signature".to_string()));
             }
        } else {
             // For now we allow unsigned (legacy) but log warning?
             // Or maybe we strictly enforce only if it looks like a signed one?
             // Given we fallback, we proceed.
             // In future, enforce strictness.
             println!("⚠️ Executing unsigned transaction (legacy path)");
        }

        // Use Accounting Engine to process transfer
        // Updated path: bank::institution_subledger::engine
        let mut entry = atlas_bank::institution_subledger::engine::AccountingEngine::process_transfer(
            &tx.from,
            &tx.to,
            tx.amount as u64,
            &tx.asset,
            tx.memo,
        ).map_err(|e| atlas_common::error::AtlasError::Other(format!("Accounting Engine Error: {}", e)))?;

        // Enrich entry with proposal metadata
        entry.entry_id = format!("entry-{}", proposal.id);
        entry.tx_hash = proposal.hash.clone();
        entry.block_height = proposal.height;
        entry.timestamp = proposal.time;

        // Apply to state
        let mut state = self.state.write().await;
        state.apply_entry(entry.clone())
            .map_err(|e| atlas_common::error::AtlasError::Other(format!("Transaction execution failed: {}", e)))?;

        Ok(entry)
    }
}
