use sha2::{Sha256, Digest};
use crate::env::proposal::Proposal;

/// Computes the SHA-256 digest of the given data and returns it as a hex string.
pub fn digest(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Computes the hash of a proposal.
///
/// The hash covers:
/// - proposer
/// - content
/// - parent
/// - height
/// - prev_hash
/// - round
/// - time
/// - state_root
///
/// It does NOT cover the signature or the ID (which is often random or derived).
pub fn compute_proposal_hash(p: &Proposal) -> String {
    let mut hasher = Sha256::new();
    
    // We serialize fields manually or use a struct view to ensure deterministic order.
    // Here we simply update the hasher with string bytes or LE bytes.
    
    hasher.update(p.proposer.0.as_bytes());
    hasher.update(p.content.as_bytes());
    if let Some(parent) = &p.parent {
        hasher.update(parent.as_bytes());
    }
    hasher.update(&p.height.to_le_bytes());
    hasher.update(p.prev_hash.as_bytes());
    hasher.update(&p.round.to_le_bytes());
    hasher.update(&p.time.to_le_bytes());
    hasher.update(p.state_root.as_bytes());

    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::proposal::Proposal;
    use crate::utils::NodeId;

    #[test]
    fn test_digest() {
        let data = b"hello world";
        let hash = digest(data);
        assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }

    #[test]
    fn test_compute_proposal_hash() {
        let p = Proposal {
            id: "prop-1".to_string(),
            proposer: NodeId("node-1".to_string()),
            content: "payload".to_string(),
            parent: None,
            height: 10,
            prev_hash: "prev".to_string(),
            round: 1,
            time: 1234567890,
            state_root: "root".to_string(),
            signature: [0u8; 64],
            public_key: vec![],
            hash: String::new(), // Should be ignored
        };

        let hash1 = compute_proposal_hash(&p);
        let hash2 = compute_proposal_hash(&p);
        
        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }
}
