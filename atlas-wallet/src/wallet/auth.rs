use super::manager::Wallet;
use crate::profile::profile_type::Profile;
use atlas_common::auth::Authenticator;

impl Authenticator for Wallet {
    fn sign(&self, _message: Vec<u8>) -> Result<Vec<u8>, String> {
        // The sign_message method needs mutable access, which `self` doesn't have here.
        // This is a design issue with the Authenticator trait expecting `&self`.
        // For now, we can't directly call the method. We will need to adjust this.
        // As a temporary workaround, this will fail if called.
        Err("Signing via Authenticator trait not implemented for stateful Wallet yet".to_string())
    }

    fn verify_with_key(&self, message: Vec<u8>, signature: &[u8; 64], public_key: &[u8]) -> Result<bool, String> {
        use ed25519_dalek::{VerifyingKey, Signature, Verifier};
        
        let key = VerifyingKey::from_bytes(public_key.try_into().map_err(|_| "Invalid key length")?)
            .map_err(|e| e.to_string())?;
        let sig = Signature::from_bytes(signature);
        
        Ok(key.verify(&message, &sig).is_ok())
    }

    fn verify(&self, message: Vec<u8>, signature: &[u8; 64]) -> Result<bool, String> {
        self.validate_message(message, signature).map(|_| true).or(Ok(false))
    }

    fn public_key(&self) -> Vec<u8> {
        if let Some(session) = self.session.as_ref() {
            let verifying_key = match &session.profile {
                Profile::Exposed(p) => p.data.public_key,
                Profile::Hidden(p) => p.data.public_key,
            };
            verifying_key.to_bytes().to_vec()
        } else {
            vec![] // Return empty vec if session is not loaded
        }
    }
}
