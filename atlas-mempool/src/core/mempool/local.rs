use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use atlas_common::transactions::{SignedTransaction, signing_bytes};
use atlas_common::error::Result;

/// In-memory Mempool implementation (Legacy/Dev).
#[derive(Debug, Default, Clone)]
pub struct LocalMempool {
    transactions: Arc<RwLock<HashMap<String, SignedTransaction>>>,
    pending: Arc<RwLock<HashMap<String, u64>>>,
    committed_cache: Arc<RwLock<std::collections::HashSet<String>>>,
    committed_order: Arc<RwLock<std::collections::VecDeque<String>>>,
}

impl LocalMempool {
    pub fn new() -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
            pending: Arc::new(RwLock::new(HashMap::new())),
            committed_cache: Arc::new(RwLock::new(std::collections::HashSet::new())),
            committed_order: Arc::new(RwLock::new(std::collections::VecDeque::new())),
        }
    }

    pub async fn add(&self, tx: SignedTransaction) -> Result<bool> {
        tx.validate_stateless().map_err(|e| atlas_common::error::AtlasError::Auth(e.to_string()))?;

        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let timestamp = tx.transaction.timestamp;
        
        if timestamp < now.saturating_sub(60) || timestamp > now + 30 {
             return Err(atlas_common::error::AtlasError::Other(format!("Timestamp invalid: {}", timestamp)));
        }

        let tx_hash = self.hash_signed_tx(&tx);
        
        {
            let committed = self.committed_cache.read().unwrap();
            if committed.contains(&tx_hash) {
                return Err(atlas_common::error::AtlasError::Other(format!("Transaction already committed: {}", tx_hash)));
            }
        }

        let mut pool = self.transactions.write().unwrap();
        if pool.contains_key(&tx_hash) {
            return Ok(false);
        }
        pool.insert(tx_hash, tx);
        Ok(true)
    }

    pub async fn mark_pending(&self, tx_hashes: &[String]) -> Result<()> {
        let mut pending = self.pending.write().unwrap();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        for hash in tx_hashes {
            pending.insert(hash.clone(), now);
        }
        Ok(())
    }

    pub async fn unmark_pending(&self, tx_hashes: &[String]) -> Result<()> {
        let mut pending = self.pending.write().unwrap();
        for hash in tx_hashes {
            pending.remove(hash);
        }
        Ok(())
    }

    pub async fn cleanup_pending(&self, allow_time_sec: u64) -> Result<usize> {
        let mut pending = self.pending.write().unwrap();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        
        let initial_len = pending.len();
        pending.retain(|_, &mut ts| now < ts + allow_time_sec);
        Ok(initial_len - pending.len())
    }

    pub async fn remove_batch(&self, tx_hashes: &[String]) -> Result<()> {
        let mut pool = self.transactions.write().unwrap();
        let mut pending = self.pending.write().unwrap();
        
        let mut committed = self.committed_cache.write().unwrap();
        let mut order = self.committed_order.write().unwrap();
        
        for hash in tx_hashes {
            pool.remove(hash);
            pending.remove(hash);
            
            if committed.insert(hash.clone()) {
                order.push_back(hash.clone());
            }
        }
        
        while order.len() > 50_000 {
            if let Some(old_hash) = order.pop_front() {
                committed.remove(&old_hash);
            }
        }
        Ok(())
    }

    pub async fn get_candidates(&self, n: usize) -> Result<Vec<(String, SignedTransaction)>> {
        let pool = self.transactions.read().unwrap();
        let pending = self.pending.read().unwrap();
        Ok(pool.iter()
            .filter(|(k, _)| !pending.contains_key(*k))
            .take(n)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }

    pub async fn get_all(&self) -> Result<Vec<String>> {
        let pool = self.transactions.read().unwrap();
        Ok(pool.keys().cloned().collect())
    }

    pub async fn len(&self) -> Result<usize> {
        let pool = self.transactions.read().unwrap();
        Ok(pool.len())
    }

    pub async fn pending_len(&self) -> Result<usize> {
        Ok(self.pending.read().unwrap().len())
    }

    fn hash_signed_tx(&self, tx: &SignedTransaction) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        let tx_bytes = signing_bytes(&tx.transaction);
        hasher.update(tx_bytes);
        hasher.update(&tx.signature);
        hex::encode(hasher.finalize())
    }
}
