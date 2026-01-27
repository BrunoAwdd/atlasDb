use atlas_common::transactions::SignedTransaction;
use atlas_common::error::Result; 

pub mod local;
pub mod redis;

use self::local::LocalMempool;
use self::redis::RedisMempool;

/// Mempool backend strategy.
#[derive(Debug, Clone)]
pub enum MempoolBackend {
    Local(LocalMempool),
    Redis(RedisMempool),
}

/// Unified Mempool wrapper.
#[derive(Debug, Clone)]
pub struct Mempool {
    backend: MempoolBackend,
}

impl Mempool {
    /// Create a new Mempool instance.
    /// If `redis_url` is Some, tries to connect to Redis.
    /// If None or falure (though failure currently errors out in builder), uses Local.
    pub fn new(redis_url: Option<String>) -> Result<Self> {
        match redis_url {
            Some(url) => {
                let bucket = RedisMempool::new(&url)?;
                Ok(Self { backend: MempoolBackend::Redis(bucket) })
            },
            None => {
                Ok(Self { backend: MempoolBackend::Local(LocalMempool::new()) })
            }
        }
    }

    /// Default constructor for testing (Local)
    pub fn default() -> Self {
        Self { backend: MempoolBackend::Local(LocalMempool::new()) }
    }

    pub async fn add(&self, tx: SignedTransaction) -> Result<bool> {
        match &self.backend {
            MempoolBackend::Local(m) => m.add(tx).await,
            MempoolBackend::Redis(m) => m.add(tx).await,
        }
    }

    pub async fn mark_pending(&self, tx_hashes: &[String]) -> Result<()> {
        match &self.backend {
            MempoolBackend::Local(m) => m.mark_pending(tx_hashes).await,
            MempoolBackend::Redis(m) => m.mark_pending(tx_hashes).await,
        }
    }

    pub async fn unmark_pending(&self, tx_hashes: &[String]) -> Result<()> {
        match &self.backend {
            MempoolBackend::Local(m) => m.unmark_pending(tx_hashes).await,
            MempoolBackend::Redis(m) => m.unmark_pending(tx_hashes).await,
        }
    }

    pub async fn cleanup_pending(&self, allow_time_sec: u64) -> Result<usize> {
        match &self.backend {
            MempoolBackend::Local(m) => m.cleanup_pending(allow_time_sec).await,
            MempoolBackend::Redis(m) => m.cleanup_pending(allow_time_sec).await,
        }
    }

    pub async fn remove_batch(&self, tx_hashes: &[String]) -> Result<()> {
        match &self.backend {
            MempoolBackend::Local(m) => m.remove_batch(tx_hashes).await,
            MempoolBackend::Redis(m) => m.remove_batch(tx_hashes).await,
        }
    }

    pub async fn get_candidates(&self, n: usize) -> Result<Vec<(String, SignedTransaction)>> {
        match &self.backend {
            MempoolBackend::Local(m) => m.get_candidates(n).await,
            MempoolBackend::Redis(m) => m.get_candidates(n).await,
        }
    }

    pub async fn get_all(&self) -> Result<Vec<String>> {
        match &self.backend {
            MempoolBackend::Local(m) => m.get_all().await,
            MempoolBackend::Redis(m) => m.get_all().await,
        }
    }

    pub async fn len(&self) -> Result<usize> {
        match &self.backend {
            MempoolBackend::Local(m) => m.len().await,
            MempoolBackend::Redis(m) => m.len().await,
        }
    }

    pub async fn pending_len(&self) -> Result<usize> {
        match &self.backend {
            MempoolBackend::Local(m) => m.pending_len().await,
            MempoolBackend::Redis(m) => m.pending_len().await,
        }
    }
}
