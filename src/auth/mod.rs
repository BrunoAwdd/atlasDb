pub mod authenticator;

pub trait Authenticator: Send + Sync {
    fn sign(&self, message: Vec<u8>, password: String) -> Result<Vec<u8>, String>;
    fn verify(&self, message: Vec<u8>, signature: &[u8; 64]) -> Result<bool, String>;
}