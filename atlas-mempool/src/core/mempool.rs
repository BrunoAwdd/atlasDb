use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use atlas_common::transactions::{SignedTransaction, signing_bytes};

/// Mempool storage backend abstraction.
/// Currently implemented as in-memory HashMap.
#[derive(Debug, Default)]
pub struct Mempool {
    // Using RwLock for thread-safe access
    // Key is TxHash, Value is SignedTransaction
    transactions: Arc<RwLock<HashMap<String, SignedTransaction>>>,
    // Store timestamp (u64) of when it was marked pending
    pending: Arc<RwLock<HashMap<String, u64>>>,
    // Cache of committed transaction hashes to prevent replay/idempotency
    committed_cache: Arc<RwLock<std::collections::HashSet<String>>>,
    // Order of commitment for pruning
    committed_order: Arc<RwLock<std::collections::VecDeque<String>>>,
}

impl Mempool {
    pub fn new() -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
            pending: Arc::new(RwLock::new(HashMap::new())),
            committed_cache: Arc::new(RwLock::new(std::collections::HashSet::new())),
            committed_order: Arc::new(RwLock::new(std::collections::VecDeque::new())),
        }
    }

    /// Adds a transaction to the mempool.
    /// Returns Ok(true) if added, Ok(false) if duplicate, Err if invalid signature or timestamp.
    pub fn add(&self, tx: SignedTransaction) -> Result<bool, String> {
        // 1. Verify Stateless (Signature, Address Derivation, Formats)
        tx.validate_stateless()?;

        // 2. Timestamp Validation (Time Window)
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let timestamp = tx.transaction.timestamp;
        
        // Reject if older than 60 seconds
        if timestamp < now.saturating_sub(60) {
            return Err(format!("Transaction too old. Timestamp: {}, Threshold: {}", timestamp, now - 60));
        }
        
        // Reject if in the future (> 30s drift)
        if timestamp > now + 30 {
             return Err(format!("Transaction from the future. Timestamp: {}, Now: {}", timestamp, now));
        }

        // 3. Add to Pool
        let tx_hash = self.hash_signed_tx(&tx);
        
        // 4. Idempotency Check (Committed Cache)
        {
            let committed = self.committed_cache.read().unwrap();
            if committed.contains(&tx_hash) {
                return Err(format!("Transaction already committed (Idempotency Rejection): {}", tx_hash));
            }
        }

        let mut pool = self.transactions.write().unwrap();
        if pool.contains_key(&tx_hash) {
            return Ok(false);
        }
        pool.insert(tx_hash, tx);
        Ok(true)
    }

    /// Marks transactions as "pending" (included in a block but not committed).
    /// They remain in the pool (to prevent duplicates) but are ignored by get_candidates.
    pub fn mark_pending(&self, tx_hashes: &[String]) {
        let mut pending = self.pending.write().unwrap();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        for hash in tx_hashes {
            pending.insert(hash.clone(), now);
        }
    }

    pub fn unmark_pending(&self, tx_hashes: &[String]) {
        let mut pending = self.pending.write().unwrap();
        for hash in tx_hashes {
            pending.remove(hash);
        }
    }

    /// Releases transactions that have been pending for too long (e.g., failed proposals).
    /// allow_time_sec: seconds before considering it "stuck" (e.g. 20s)
    pub fn cleanup_pending(&self, allow_time_sec: u64) -> usize {
        let mut pending = self.pending.write().unwrap();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        
        let initial_len = pending.len();
        pending.retain(|_, &mut ts| now < ts + allow_time_sec);
        initial_len - pending.len()
    }

    /// Removes a list of transactions (e.g., after they are included in a block).
    /// Moves them to committed_cache for idempotency.
    pub fn remove_batch(&self, tx_hashes: &[String]) {
        println!("Removing {} transactions from mempool", tx_hashes.len());
        let mut pool = self.transactions.write().unwrap();
        let mut pending = self.pending.write().unwrap();
        
        let mut committed = self.committed_cache.write().unwrap();
        let mut order = self.committed_order.write().unwrap();
        
        for hash in tx_hashes {
            pool.remove(hash);
            pending.remove(hash);
            
            // Add to Idempotency Cache
            if committed.insert(hash.clone()) {
                order.push_back(hash.clone());
            }
        }
        
        // Cache Pruning (Keep Max 50,000 hashes ~ 3MB RAM)
        while order.len() > 50_000 {
            if let Some(old_hash) = order.pop_front() {
                committed.remove(&old_hash);
            }
        }
    }

    /// Returns `n` transactions to be included in the next block.
    /// Simple FIFO/random selection for now.
    pub fn get_candidates(&self, n: usize) -> Vec<(String, SignedTransaction)> {
        let pool = self.transactions.read().unwrap();
        let pending = self.pending.read().unwrap();
        pool.iter()
            .filter(|(k, _)| !pending.contains_key(*k))
            .take(n)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn get_all(&self) -> Vec<String> {
        let pool = self.transactions.read().unwrap();
        pool.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        let pool = self.transactions.read().unwrap();
        pool.len()
    }

    pub fn pending_len(&self) -> usize {
        self.pending.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // Helper to hash signed transaction
    // We include signature in the hash to differentiate same tx signed differently
    fn hash_signed_tx(&self, tx: &SignedTransaction) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        // Hash the transaction content
        let tx_bytes = signing_bytes(&tx.transaction);
        hasher.update(tx_bytes);
        // Hash the signature
        hasher.update(&tx.signature);
        
        let result = hasher.finalize();
        hex::encode(result)
    }
}
