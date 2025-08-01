use super::Authenticator;
use std::sync::Arc;

pub struct SimpleAuthenticator {
    pub key: Arc<Vec<u8>>,
}

impl SimpleAuthenticator {
    pub fn new(key: Vec<u8>) -> Self {
        SimpleAuthenticator { key: Arc::new(key) }
    }
}

impl Authenticator for SimpleAuthenticator {
    fn sign(&self, message: Vec<u8>, _password: String) -> Result<Vec<u8>, String> {
        // Mock: assinatura é apenas message + key
        let mut fake_signature = Vec::new();
        fake_signature.extend_from_slice(&message);
        fake_signature.extend_from_slice(&self.key);
        Ok(fake_signature)
    }

    fn verify(&self, message: Vec<u8>, received_signature: &[u8; 64]) -> Result<bool, String> {
        // Mock: gera a mesma "assinatura" e compara com a recebida
        let mut expected_signature = Vec::new();
        expected_signature.extend_from_slice(&message);
        expected_signature.extend_from_slice(&self.key);

        Ok(&expected_signature == received_signature)
    }
}
