use std::sync::Arc;

use crate::auth::Authenticator;

pub struct SimpleAuthenticator {
    pub key: Arc<Vec<u8>>,
}

impl SimpleAuthenticator {
    pub fn new(key: Vec<u8>) -> Self {
        Self { key: Arc::new(key) }
    }
}

// Digest fake de 32 bytes (NÃO criptográfico!)
// Só pra mock: mistura som + xor + rotate em 32 "células".
fn mock_digest32(data: &[u8]) -> [u8; 32] {
    let mut acc = [0u8; 32];

    for (i, &b) in data.iter().enumerate() {
        let idx = i % 32;
        acc[idx] = acc[idx]
            .wrapping_add(b)
            .rotate_left((b & 0x07) as u32)
            ^ b.wrapping_mul(31);

        acc[(idx + 13) % 32] ^= b.wrapping_add((i as u8).wrapping_mul(17));
    }

    // umas rodadas extras de mistura
    for _ in 0..8 {
        for i in 0..32 {
            let j = (i * 7 + 1) % 32;
            acc[i] = acc[i]
                .wrapping_add(acc[j])
                .rotate_left((acc[j] & 0x0F) as u32)
                ^ (i as u8);
        }
    }

    acc
}

impl Authenticator for SimpleAuthenticator {
    fn sign(&self, message: Vec<u8>, _password: String) -> Result<Vec<u8>, String> {
        // digest(message || key) -> 32 bytes -> HEX (64 ASCII)
        let mut buf = Vec::with_capacity(message.len() + self.key.len());
        buf.extend_from_slice(&message);
        buf.extend_from_slice(&self.key[..]);

        let digest = mock_digest32(&buf);
        let hex_str = hex::encode(digest); // 64 chars ASCII
        Ok(hex_str.into_bytes())           // Vec<u8> len == 64
    }

    fn verify(&self, message: Vec<u8>, received_signature: &[u8; 64]) -> Result<bool, String> {
        let mut buf = Vec::with_capacity(message.len() + self.key.len());
        buf.extend_from_slice(&message);
        buf.extend_from_slice(&self.key[..]);

        let expected_hex = hex::encode(mock_digest32(&buf)); // 64 chars

        Ok(expected_hex.as_bytes() == &received_signature[..])
    }
}
