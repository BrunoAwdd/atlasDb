use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use atlas_common::transaction::{Transaction, SignedTransaction, signing_bytes};

/// Mempool storage backend abstraction.
/// Currently implemented as in-memory HashMap.
#[derive(Debug, Default)]
pub struct Mempool {
    // Using RwLock for thread-safe access
    // Key is TxHash, Value is SignedTransaction
    transactions: Arc<RwLock<HashMap<String, SignedTransaction>>>,
}

impl Mempool {
    pub fn new() -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Adds a transaction to the mempool.
    /// Returns Ok(true) if added, Ok(false) if duplicate, Err if invalid signature.
    pub fn add(&self, tx: SignedTransaction) -> Result<bool, String> {
        // 1. Verify Signature
        use atlas_common::transaction::signing_bytes;
        use ed25519_dalek::{Verifier, VerifyingKey, Signature};

        let verifying_key = VerifyingKey::from_bytes(tx.public_key.as_slice().try_into().unwrap_or(&[0u8; 32]))
            .map_err(|e| format!("Invalid public key: {}", e))?;
        
        // Note: signature has to be 64 bytes
        let signature_bytes: [u8; 64] = tx.signature.as_slice().try_into()
            .map_err(|_| "Invalid signature length".to_string())?;

        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|e| format!("Invalid signature format: {}", e))?;

        let msg = signing_bytes(&tx.transaction);

        if verifying_key.verify(&msg, &signature).is_err() {
            return Err("Invalid transaction signature".to_string());
        }

        // 2. Add to Pool
        let tx_hash = self.hash_signed_tx(&tx);
        let mut pool = self.transactions.write().unwrap();
        if pool.contains_key(&tx_hash) {
            return Ok(false);
        }
        pool.insert(tx_hash, tx);
        Ok(true)
    }

    /// Removes a list of transactions (e.g., after they are included in a block).
    pub fn remove_batch(&self, tx_hashes: &[String]) {
        let mut pool = self.transactions.write().unwrap();
        for hash in tx_hashes {
            pool.remove(hash);
        }
    }

    /// Returns `n` transactions to be included in the next block.
    /// Simple FIFO/random selection for now.
    pub fn get_candidates(&self, n: usize) -> Vec<(String, SignedTransaction)> {
        let pool = self.transactions.read().unwrap();
        pool.iter().take(n).map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    pub fn len(&self) -> usize {
        let pool = self.transactions.read().unwrap();
        pool.len()
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
