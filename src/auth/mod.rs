pub trait Authenticator {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, String> ;
    fn verify(&self, message: &[u8], signature: &[u8]) -> Result<bool, String>;
}