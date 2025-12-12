use sha2::{Digest, Sha256};

/// Calculates a deterministic mock state root for development purposes.
/// 
/// Formula: SHA256(height + prev_hash + "dev")
pub fn calculate_mock_state_root(height: u64, prev_hash: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(height.to_be_bytes());
    hasher.update(prev_hash.as_bytes());
    hasher.update(b"dev");
    format!("{:x}", hasher.finalize())
}
