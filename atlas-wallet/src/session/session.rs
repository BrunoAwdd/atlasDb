use std::collections::HashMap;
use crate::errors::NimbleError;
use crate::identity::errors::IdentityError;
use crate::identity::identity::{Identity, IdentityBundle};
use crate::profile::profile_type::{Profile, ProfileType,};
use ed25519_dalek::SigningKey;
use atlas_common::address::profile_address::ProfileAddress;
use atlas_common::transactions::{payload::TransferPayload, TransferRequest};

pub struct Session {
    pub identity: Identity,
    pub profile: Profile,
    // verifying_key: Option<ed25519_dalek::VerifyingKey>, // Not strictly needed if we have SigningKey, but useful for validation
    signing_key: Option<ed25519_dalek::SigningKey>,
    secrets: HashMap<String, SigningKey>, // Cache keys: "exposed" -> Key, "hidden" -> Key
}

impl Session {
    /// Cria uma nova sessão com um Profile e Identity carregados
    pub fn new(identity: Identity, secrets: HashMap<String, SigningKey>) -> Result<Self, NimbleError> {
        let profile = Profile::Exposed(identity.clone().exposed);
        // Set initial signing key for exposed profile
        let signing_key = secrets.get("exposed").cloned();
        
        Ok(Self { identity, profile, signing_key, secrets })
    }

    pub fn from_bundle(bundle: IdentityBundle) -> Result<Self, NimbleError> {
        let mut secrets = HashMap::new();
        
        let sk_exposed = SigningKey::from_bytes(&bundle.sk_exposed);
        let sk_hidden = SigningKey::from_bytes(&bundle.sk_hidden);
        
        secrets.insert("exposed".to_string(), sk_exposed);
        secrets.insert("hidden".to_string(), sk_hidden);

        let session = Self::new(bundle.identity, secrets)?;
        Ok(session)
    }

    pub fn switch_profile(&mut self) -> Result<&dyn ProfileType, IdentityError> {
        // No need to clear sensitive data as we are intentionally keeping it in memory
        
        let profile_id = self.profile.id();

        match profile_id {
            "hidden" => {
                self.profile = Profile::Exposed(self.identity.exposed.clone());
            }
            "exposed" => {
                self.profile = Profile::Hidden(self.identity.hidden.clone());   
            }
            _ => {
                return Err(IdentityError::ProfileNotFound(profile_id.to_string()));
            }
        }
        
        // Update active signing key based on new profile
        let new_id = self.profile.id();
        self.signing_key = self.secrets.get(new_id).cloned();

        Ok(match &self.profile {
            Profile::Hidden(p) => p,
            Profile::Exposed(p) => p,
        })
    }

    pub fn sign_message(&self, message: &[u8]) -> Result<[u8;64], NimbleError> {
        let secret = self.signing_key
            .clone()
            .ok_or_else(|| NimbleError::from("Session key not available (should be loaded)"))?;
        let sig = self.profile.sign(secret, message)?;
        Ok(sig)
    }

    pub fn create_signed_transfer(
        &mut self,
        to_address: String,
        amount: u64,
        memo: Option<String>,
        nonce: u64,
    ) -> Result<TransferRequest, NimbleError> {
        if self.profile.address().as_str() == to_address {            
            return Err(NimbleError::from("Same address transaction not allowed"));
        }
        let timestamp = atlas_common::utils::time::current_time();

        // Create Transaction struct exactly as Ledger expects for verification
        let transaction = atlas_common::transactions::Transaction {
            from: self.profile.address().as_str().to_string(),
            to: to_address.clone(),
            amount: amount as u128,
            asset: "BRL".to_string(),
            nonce,
            timestamp,
            memo: memo.clone(),
        };

        // Use the same signing bytes logic as Ledger: bincode::serialize(&transaction)
        let msg = atlas_common::transactions::signing_bytes(&transaction);

        // No unlock_key needed, key should be in self.signing_key
        let signature = self.sign_message(&msg)?;
        
        let request = TransferRequest::build_signed_request(
            self.profile.address().as_str().to_string(),
            to_address,
            amount,
            signature,
            memo,
            timestamp,
            nonce,
        )?;

        Ok(request)
    }

    // validate_transfer and validate_message can reconstruct verifying key from signing key or use profile public key
    // For now, let's remove strict Session expiration and just use profile methods.
    // Profile has validate_transfer methods but they need verifying key.
    // We can get verifying key from signing key if present, or from profile data?
    // Profile struct usually holds public key.
    
    // Let's implement helper to get verifying key always
    fn get_verifying_key(&self) -> Result<ed25519_dalek::VerifyingKey, NimbleError> {
        if let Some(sk) = &self.signing_key {
            return Ok(ed25519_dalek::VerifyingKey::from(sk));
        }
        // Fallback to profile public key if we want, but for Session intended use (signing), we need SK.
        // For validation only, we might use profile PK.
        // Let's try to use SigningKey since we expect it to be there.
        Err(NimbleError::from("Signing key not available"))
    }

    pub fn validate_transfer(&self, transfer: TransferRequest) -> Result<(), NimbleError> {
        let vk = self.get_verifying_key()?;
        self.profile.validate_transfer(transfer, vk)?;
        Ok(())
    }

    pub fn validate_message(&self, msg: Vec<u8>, signature: &[u8; 64]) -> Result<(), NimbleError> {
        let vk = self.get_verifying_key()?;
        self.profile.validate_message(msg, signature, vk)?;
        Ok(())
    }

    pub fn clear_sensitive(&mut self) {
        // If we want to really clear, we clear the secrets map.
        self.signing_key = None;
        self.secrets.clear();
    }

    pub fn get_public_key(&self) -> Option<Vec<u8>> {
         self.signing_key.as_ref().map(|sk| ed25519_dalek::VerifyingKey::from(sk).as_bytes().to_vec())
    }
    
    pub fn clear(&mut self) {
        self.profile.zeroize();
        self.clear_sensitive();
    }
}
// Tests need update too
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{identity::identity::generate, profile::profile_type::ProfileType};

    fn create_test_session() -> Session {
        let seed = [42u8; 32];
        let password = "test-password".to_string();
        let bundle = generate(&seed, password.clone()).unwrap();

        let session = Session::from_bundle(bundle).unwrap();
        session
    }

    #[test]
    fn test_session_creation() {
        let session = create_test_session();
        assert_eq!(session.profile.id(), "exposed");
        assert!(session.identity.exposed.is_public());
        assert!(!session.identity.hidden.is_public());
        // Should hold keys
        assert!(session.signing_key.is_some());
    }
    
    #[test]
    fn test_switch_profile() {
        let mut session = create_test_session();
        session.switch_profile().unwrap();
        assert_eq!(session.profile.id(), "hidden");
        assert_ne!(session.profile.id(), "exposed");
        // Should still hold keys (switched)
        assert!(session.signing_key.is_some());
    }

    #[test]
    fn test_session_sign_message() {
        let session = create_test_session();
        let message = b"hello nimble";

        // No need to unlock
        let signature = session.sign_message(message)
            .expect("Signing should succeed");

        assert_eq!(signature.len(), 64);
    }

    #[test]
    fn test_session_clear() {
        let mut session = create_test_session();

        session.clear();

        // Verifica que campos sensíveis foram zerados
        assert!(session.profile.is_cleared(), "Profile should be cleared");
        assert!(session.secrets.is_empty(), "Secrets should be cleared");
        assert!(session.signing_key.is_none(), "Signing key should be cleared");
    }
}
