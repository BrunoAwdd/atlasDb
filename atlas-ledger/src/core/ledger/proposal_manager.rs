use atlas_common::env::proposal::Proposal;
use atlas_common::error::Result;
use atlas_common::transactions::{SignedTransaction, signing_bytes};
use sha2::{Sha256, Digest};
use crate::Ledger;

impl Ledger {
    pub async fn append_proposal(&self, proposal: &Proposal) -> Result<()> {
        let mut binlog = self.binlog.write().await;
        let mut index = self.index.write().await;

        let (file_id, offset, len) = binlog.append(proposal).await?;
        
        // Extract inner transaction hash(es) for idempotency index
        let tx_hashes: Vec<String> = if let Ok(batch) = serde_json::from_str::<Vec<SignedTransaction>>(&proposal.content) {
             batch.iter().map(|signed_tx| {
                 let mut hasher = Sha256::new();
                 hasher.update(signing_bytes(&signed_tx.transaction));
                 hasher.update(&signed_tx.signature);
                 hex::encode(hasher.finalize())
             }).collect()
        } else if let Ok(signed_tx) = serde_json::from_str::<SignedTransaction>(&proposal.content) {
            let mut hasher = Sha256::new();
            hasher.update(signing_bytes(&signed_tx.transaction));
            hasher.update(&signed_tx.signature);
            vec![hex::encode(hasher.finalize())]
        } else {
             // Fallback: use proposal hash if parsing fails (legacy)
             vec![proposal.hash.clone()]
        };

        for hash in tx_hashes {
            index.index_proposal(&proposal.id, &hash, file_id, offset, len)?;
        }
        
        // CRITICAL FIX: Update State immediately!
        // Drop locks before executing to avoid potential deadlock issues (though execute takes its own locks)
        drop(binlog);
        drop(index);

        match self.execute_transaction(proposal, true).await {
            Ok(_) => tracing::info!("✅ State updated for proposal {}", proposal.id),
            Err(e) => tracing::error!("❌ Failed to update state for proposal {}: {}", proposal.id, e),
        }
        
        Ok(())
    }

    pub async fn exists_transaction(&self, hash: &str) -> Result<bool> {
        let index = self.index.read().await;
        index.exists_tx(hash)
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
}
