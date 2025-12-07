use super::Authenticator;
use async_trait::async_trait;
use ed25519_dalek::{Signer, SigningKey, Verifier, Signature, VerifyingKey};

pub struct Ed25519Authenticator {
    keypair: SigningKey,
}

impl Ed25519Authenticator {
    pub fn new(keypair: SigningKey) -> Self {
        Self { keypair }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let keypair = SigningKey::from_bytes(bytes.try_into().map_err(|_| "Invalid key length")?);
        Ok(Self { keypair })
    }
}

#[async_trait]
impl Authenticator for Ed25519Authenticator {
    fn sign(&self, message: Vec<u8>) -> Result<Vec<u8>, String> {
        let signature = self.keypair.sign(&message);
        Ok(signature.to_vec())
    }

    fn verify(&self, message: Vec<u8>, signature: &[u8; 64]) -> Result<bool, String> {
        let verifying_key = self.keypair.verifying_key();
        let signature = Signature::from_slice(signature).map_err(|e| e.to_string())?;
        
        match verifying_key.verify(&message, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn verify_with_key(&self, message: Vec<u8>, signature: &[u8; 64], public_key: &[u8]) -> Result<bool, String> {
        let verifying_key = VerifyingKey::from_bytes(public_key.try_into().map_err(|_| "Invalid public key length")?)
            .map_err(|e| e.to_string())?;
        let signature = Signature::from_slice(signature).map_err(|e| e.to_string())?;
        
        match verifying_key.verify(&message, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn public_key(&self) -> Vec<u8> {
        self.keypair.verifying_key().to_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn test_ed25519_signing_and_verification() {
        let mut csprng = OsRng;
        let keypair = SigningKey::generate(&mut csprng);
        let auth = Ed25519Authenticator::new(keypair);

        let message = b"hello world";
        let signature = auth.sign(message.to_vec()).expect("Signing failed");

        assert_eq!(signature.len(), 64);

        let valid = auth.verify(message.to_vec(), &signature).expect("Verification failed");
        assert!(valid, "Signature should be valid");

        let invalid_valid = auth.verify(b"wrong message".to_vec(), &signature).expect("Verification failed");
        assert!(!invalid_valid, "Signature should be invalid for wrong message");
    }
}
