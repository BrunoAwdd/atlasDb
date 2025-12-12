use rs_merkle::{MerkleTree, algorithms::Sha256};

/// Calculates the Merkle Root of a list of leaves.
/// Leaves should be pre-hashed (32 bytes).
pub fn calculate_merkle_root(leaves: &[Vec<u8>]) -> String {
    if leaves.is_empty() {
        return "0000000000000000000000000000000000000000000000000000000000000000".to_string();
    }

    // Convert Vec<u8> to [u8; 32]
    let leaves_arr: Vec<[u8; 32]> = leaves.iter()
        .map(|l| {
            let mut arr = [0u8; 32];
            let len = std::cmp::min(l.len(), 32);
            arr[..len].copy_from_slice(&l[..len]);
            arr
        })
        .collect();

    let tree = MerkleTree::<Sha256>::from_leaves(&leaves_arr);
    
    if let Some(root) = tree.root_hex() {
        root
    } else {
        "0000000000000000000000000000000000000000000000000000000000000000".to_string()
    }
}
