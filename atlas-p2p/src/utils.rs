
use libp2p::PeerId;
use std::str::FromStr;

/// Helper to convert a Libp2p PeerId string into an Atlas Base58 Address.
/// Assumes Ed25519 Identity Keys.
pub fn node_id_to_address(node_id_str: &str) -> Option<String> {
    let peer_id = PeerId::from_str(node_id_str).ok()?;
    let bytes = peer_id.to_bytes();

    // Check for Ed25519 Identity Key pattern:
    // 0x00 (Identity Code)
    // 0x24 (Length 36)
    // 0x08 0x01 (KeyType Ed25519)
    // 0x12 0x20 (Field Data, Length 32)
    // Total prefix: 6 bytes [0, 36, 8, 1, 18, 32]
    if bytes.len() == 38 && bytes.starts_with(&[0x00, 0x24, 0x08, 0x01, 0x12, 0x20]) {
         let pub_key_bytes = &bytes[6..];
         return Some(bs58::encode(pub_key_bytes).into_string());
    }

    // Do not panic or log warning here, let caller handle failure
    None
}
