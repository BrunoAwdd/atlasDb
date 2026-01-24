use std::sync::Arc;
use tokio::sync::RwLock;
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
}
