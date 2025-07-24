pub mod authenticator;

pub trait Authenticator: Send + Sync {
    fn sign(&self, message: Vec<u8>, password: String) -> Result<Vec<u8>, String>;
    fn verify(&self, message: Vec<u8>, signature: Vec<u8>) -> Result<(bool), String>;
}