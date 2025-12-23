use std::sync::Arc;
use tokio::sync::RwLock;
use atlas_common::error::Result;
use crate::core::runtime::{binlog::Binlog, index::Index};
use crate::core::ledger::state::State;

#[derive(Debug)]
pub struct Ledger {
    pub binlog: Arc<RwLock<Binlog>>,
    pub index: Arc<RwLock<Index>>,
    pub state: Arc<RwLock<State>>,
}

impl Ledger {
    pub async fn new(data_dir: &str) -> Result<Self> {
        let binlog = Binlog::new(data_dir).await?;
        let index = Index::new(data_dir)?;
        
        let ledger = Self {
            binlog: Arc::new(RwLock::new(binlog)),
            index: Arc::new(RwLock::new(index)),
            state: Arc::new(RwLock::new(State::new())),
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

    pub async fn get_balance(&self, address: &str, asset: &str) -> Result<u64> {
        let state = self.state.read().await;
        if let Some(account) = state.accounts.get(address) {
            let bal = *account.balances.get(asset).unwrap_or(&0);
            Ok(bal.try_into().unwrap_or(u64::MAX))
        } else {
            Ok(0)
        }
    }
}
