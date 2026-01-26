pub mod ed25519;

pub trait Authenticator: Send + Sync {
    fn sign(&self, message: Vec<u8>) -> Result<Vec<u8>, String>;
    fn verify(&self, message: Vec<u8>, signature: &[u8; 64]) -> Result<bool, String>;
    fn verify_with_key(&self, message: Vec<u8>, signature: &[u8; 64], public_key: &[u8]) -> Result<bool, String>;
    fn public_key(&self) -> Vec<u8>;
}