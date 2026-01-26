
use libp2p::PeerId;
use std::str::FromStr;

// Helper to convert a Libp2p PeerId string into an Atlas Bech32 Address (nbex...).
// Assumes Ed25519 Identity Keys.
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
         
         // Use strict conversion to Bech32 via atlas-common
         use ed25519_dalek::VerifyingKey;
         use atlas_common::address::address::Address;

         if let Ok(bytes_array) = pub_key_bytes.try_into() {
             if let Ok(vk) = VerifyingKey::from_bytes(bytes_array) {
                 return Address::address_from_pk(&vk, "nbex").ok();
             }
         }
    }
    None
}
