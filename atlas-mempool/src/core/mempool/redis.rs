
use atlas_common::transactions::{SignedTransaction, signing_bytes};
use redis::{AsyncCommands, Client};
use atlas_common::error::Result;
use sha2::{Sha256, Digest};

/// Mempool storage backend using Redis.
#[derive(Debug, Clone)]
pub struct RedisMempool {
    client: Client,
}

impl RedisMempool {
    pub fn new(redis_url: &str) -> Result<Self> {
        let client = Client::open(redis_url).map_err(|e| atlas_common::error::AtlasError::Other(e.to_string()))?;
        Ok(Self { client })
    }

    /// Adds a transaction to the mempool.
    /// Returns Ok(true) if added, Ok(false) if duplicate, Err if invalid.
    pub async fn add(&self, tx: SignedTransaction) -> Result<bool> {
        // 1. Verify Stateless
        if let Err(e) = tx.validate_stateless() {
            return Err(atlas_common::error::AtlasError::Auth(e.to_string()));
        }

        // 2. Timestamp Validation
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let timestamp = tx.transaction.timestamp;
        if timestamp < now.saturating_sub(60) || timestamp > now + 30 {
             return Err(atlas_common::error::AtlasError::Other(format!("Timestamp invalid: {}", timestamp)));
        }

        let tx_hash = self.hash_signed_tx(&tx);
        let mut con = self.client.get_multiplexed_async_connection().await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;

        // 3. Idempotency Check (committed set)
        let is_committed: bool = con.sismember("mempool:committed", &tx_hash).await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
        
        if is_committed {
            return Err(atlas_common::error::AtlasError::Other(format!("Transaction already committed: {}", tx_hash)));
        }

        // 4. Add to Pool (Hash: tx_hash -> json)
        let tx_json = serde_json::to_string(&tx).unwrap();
        let is_new: bool = con.hset_nx("mempool:txs", &tx_hash, &tx_json).await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;

        Ok(is_new)
    }

    pub async fn mark_pending(&self, tx_hashes: &[String]) -> Result<()> {
        let mut con = self.client.get_multiplexed_async_connection().await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
        
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        
        for hash in tx_hashes {
            let _: () = con.hset("mempool:pending", hash, now).await
                .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn unmark_pending(&self, tx_hashes: &[String]) -> Result<()> {
        let mut con = self.client.get_multiplexed_async_connection().await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
        
        for hash in tx_hashes {
             let _: () = con.hdel("mempool:pending", hash).await
                .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn cleanup_pending(&self, allow_time_sec: u64) -> Result<usize> {
        let mut con = self.client.get_multiplexed_async_connection().await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;

        let pending: std::collections::HashMap<String, u64> = con.hgetall("mempool:pending").await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;

        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let mut removed = 0;

        for (hash, ts) in pending {
            if now > ts + allow_time_sec {
                let _: () = con.hdel("mempool:pending", &hash).await
                    .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
                removed += 1;
            }
        }
        Ok(removed)
    }

    pub async fn remove_batch(&self, tx_hashes: &[String]) -> Result<()> {
        let mut con = self.client.get_multiplexed_async_connection().await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;

        for hash in tx_hashes {
            // Remove from pool and pending
            let _: () = con.hdel("mempool:txs", hash).await
                 .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
            let _: () = con.hdel("mempool:pending", hash).await
                 .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;

            // Add to Committed
            let _: () = con.sadd("mempool:committed", hash).await
                 .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
            
            // Manage Pruning (approximate via random removal or just TTL? TTL is better for Redis)
            // For now, simpler: do nothing or use a ZSET with timestamp if we want strict pruning order.
            // Let's rely on Redis not running out of memory for MVP or assume external cleanup.
        }
        Ok(())
    }

    pub async fn get_candidates(&self, n: usize) -> Result<Vec<(String, SignedTransaction)>> {
        let mut con = self.client.get_multiplexed_async_connection().await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;

        let all_txs: std::collections::HashMap<String, String> = con.hgetall("mempool:txs").await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
        
        let pending: std::collections::HashMap<String, u64> = con.hgetall("mempool:pending").await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;

        let mut candidates = Vec::new();
        for (hash, json) in all_txs {
            if candidates.len() >= n {
                break;
            }
            if !pending.contains_key(&hash) {
                if let Ok(tx) = serde_json::from_str::<SignedTransaction>(&json) {
                    candidates.push((hash, tx));
                }
            }
        }
        Ok(candidates)
    }

    pub async fn get_all(&self) -> Result<Vec<String>> {
        let mut con = self.client.get_multiplexed_async_connection().await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
        
        let keys: Vec<String> = con.hkeys("mempool:txs").await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
        Ok(keys)
    }

    pub async fn len(&self) -> Result<usize> {
        let mut con = self.client.get_multiplexed_async_connection().await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
        
        let len: usize = con.hlen("mempool:txs").await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
        Ok(len)
    }

    pub async fn pending_len(&self) -> Result<usize> {
        let mut con = self.client.get_multiplexed_async_connection().await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
        
        let len: usize = con.hlen("mempool:pending").await
            .map_err(|e| atlas_common::error::AtlasError::Storage(e.to_string()))?;
        Ok(len)
    }

    fn hash_signed_tx(&self, tx: &SignedTransaction) -> String {
        let mut hasher = Sha256::new();
        let tx_bytes = signing_bytes(&tx.transaction);
        hasher.update(tx_bytes);
        hasher.update(&tx.signature);
        hex::encode(hasher.finalize())
    }
}
