pub mod binlog;
pub mod index;

use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::env::proposal::Proposal;
use atlas_sdk::env::consensus::types::ConsensusResult;
use crate::error::Result;

#[derive(Debug)]
pub struct Ledger {
    binlog: Arc<RwLock<binlog::Binlog>>,
    index: Arc<RwLock<index::Index>>,
}

impl Ledger {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        std::fs::create_dir_all(path)?;

        let binlog = binlog::Binlog::new(path.join("binlog"))?;
        let index = index::Index::new(path.join("index"))?;

        Ok(Self {
            binlog: Arc::new(RwLock::new(binlog)),
            index: Arc::new(RwLock::new(index)),
        })
    }

    pub async fn append_proposal(&self, proposal: &Proposal) -> Result<()> {
        let mut binlog = self.binlog.write().await;
        let mut index = self.index.write().await;

        let (file_id, offset, len) = binlog.append_proposal(proposal)?;
        index.index_proposal(&proposal.id, proposal.height, file_id, offset, len)?;
        
        // Also index by height if needed, or other attributes
        Ok(())
    }

    pub async fn get_proposal(&self, id: &str) -> Result<Option<Proposal>> {
        let index = self.index.read().await;
        if let Some((file_id, offset, len)) = index.get_proposal_location(id)? {
            let binlog = self.binlog.read().await;
            return Ok(Some(binlog.read_proposal(file_id, offset, len)?));
        }
        Ok(None)
    }

    pub async fn get_proposals_after(&self, height: u64) -> Result<Vec<Proposal>> {
        let index = self.index.read().await;
        let ids = index.get_ids_after_height(height)?;
        
        // Release index lock before reading binlog to avoid potential deadlocks (though unlikely here)
        drop(index);

        let mut proposals = Vec::new();
        for id in ids {
            if let Some(prop) = self.get_proposal(&id).await? {
                proposals.push(prop);
            }
        }
        Ok(proposals)
    }
}
