pub mod binlog;
pub mod index;
pub mod storage;

use std::sync::Arc;
use tokio::sync::RwLock;
use atlas_common::env::proposal::Proposal;
use atlas_common::error::Result;

#[derive(Debug)]
pub struct Ledger {
    binlog: Arc<RwLock<binlog::Binlog>>,
    index: Arc<RwLock<index::Index>>,
}

impl Ledger {
    pub async fn new(data_dir: &str) -> Result<Self> {
        let binlog = binlog::Binlog::new(data_dir).await?;
        let index = index::Index::new(data_dir)?;
        
        Ok(Self {
            binlog: Arc::new(RwLock::new(binlog)),
            index: Arc::new(RwLock::new(index)),
        })
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

    pub async fn get_proposals_after(&self, _height: u64) -> Result<Vec<Proposal>> {
        Ok(Vec::new())
    }
}
