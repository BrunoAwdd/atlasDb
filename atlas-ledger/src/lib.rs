pub mod binlog;
pub mod index;
pub mod storage;
pub mod state;


use std::sync::Arc;
use tokio::sync::RwLock;
use atlas_common::env::proposal::Proposal;
use atlas_common::error::Result;

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
        
        Ok(Self {
            binlog: Arc::new(RwLock::new(binlog)),
            index: Arc::new(RwLock::new(index)),
            state: Arc::new(RwLock::new(state::State::new())),
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
    pub async fn execute_transaction(&self, proposal: &Proposal) -> Result<state::entry::LedgerEntry> {
        // Parse transaction from proposal content (JSON)
        #[derive(serde::Deserialize)]
        struct Transaction {
            from: String,
            to: String,
            amount: u128,
            asset: String,
            memo: Option<String>,
        }

        let tx: Transaction = serde_json::from_str(&proposal.content)
            .map_err(|e| atlas_common::error::AtlasError::Other(format!("Invalid transaction format: {}", e)))?;

        // Create LedgerEntry (Double Entry)
        let legs = vec![
            state::entry::Leg {
                account: tx.from.clone(),
                asset: tx.asset.clone(),
                kind: state::entry::LegKind::Debit,
                amount: tx.amount,
            },
            state::entry::Leg {
                account: tx.to.clone(),
                asset: tx.asset.clone(),
                kind: state::entry::LegKind::Credit,
                amount: tx.amount,
            },
        ];

        let entry = state::entry::LedgerEntry::new(
            format!("entry-{}", proposal.id),
            legs,
            proposal.hash.clone(),
            proposal.height,
            proposal.time,
            tx.memo,
        );

        // Apply to state
        let mut state = self.state.write().await;
        state.apply_entry(entry.clone())
            .map_err(|e| atlas_common::error::AtlasError::Other(format!("Transaction execution failed: {}", e)))?;

        Ok(entry)
    }
}
