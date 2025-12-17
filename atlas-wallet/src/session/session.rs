use std::time::{Duration, Instant};

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
    verifying_key: Option<ed25519_dalek::VerifyingKey>,
    signing_key: Option<ed25519_dalek::SigningKey>,
    unlocked_at: Option<Instant>
}

impl Session {
    /// Cria uma nova sessão com um Profile e Identity carregados
    pub fn new(identity: Identity) -> Result<Self, NimbleError> {
        let profile = Profile::Exposed(identity.clone().exposed);
        Ok(Self { identity, profile, verifying_key: None, signing_key: None, unlocked_at: None })
    }

    pub fn from_bundle(bundle: IdentityBundle) -> Result<Self, NimbleError> {
        let session = Self::new(bundle.identity)?;
        Ok(session)
    }

    pub fn switch_profile(&mut self) -> Result<&dyn ProfileType, IdentityError> {
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

    Ok(match &self.profile {
        Profile::Hidden(p) => p,
        Profile::Exposed(p) => p,
    })
}
    /// Exemplo de função para assinar uma mensagem
    pub fn sign_message(&self, message: &[u8]) -> Result<[u8;64], NimbleError> {
        let secret = self.signing_key
            .clone()
            .ok_or_else(|| NimbleError::from("Session not unlocked"))?;
        let sig = self.profile.sign(secret, message)?;
        Ok(sig)
    }

    pub fn create_signed_transfer(
        &mut self,
        to_address: String,
        amount: u64,
        password: String,
        memo: Option<String>,
        nonce: u64,
    ) -> Result<TransferRequest, NimbleError> {
        if self.profile.address().as_str() == to_address {            
            return Err(NimbleError::from("Same address transaction not allowed"));
        }
        let payload = TransferPayload::new(&self.profile.address().as_str(), to_address.clone(), amount, nonce);

        self.unlock_key(password)?;

        let signature = self.sign_message(payload.to_string().as_bytes())?;

        let request = TransferRequest::build_signed_request(
            self.profile.address().as_str().to_string(),
            to_address,
            amount,
            signature,
            memo,
            payload.timestamp,
            nonce,
        )?;

        Ok(request)
    }

    fn set_signing_key(&mut self, secret: SigningKey) {
        self.verifying_key = Some(ed25519_dalek::VerifyingKey::from(&secret));
        self.signing_key = Some(secret);
        self.unlocked_at = Some(Instant::now().checked_add(Duration::from_secs(600)).expect("failed to add duration to now"));
    }

    fn is_expired(&self) -> bool {
        match self.unlocked_at {
            Some(instant) => instant.elapsed() > Duration::from_secs(600),
            None => true,
        }
    }

    pub fn unlock_key(&mut self, password: String) -> Result<(), NimbleError>  {
        let secret = if self.is_expired() {
            self.profile.get_signing_key(password)?
        } else {
            self.signing_key
                .clone()
                .ok_or_else(|| NimbleError::from("Session not unlocked"))?
        };
        
        self.set_signing_key(secret);

        Ok(())
    }

    pub fn validate_transfer(&self, transfer: TransferRequest) -> Result<(), NimbleError> {
        if self.is_expired() {
            return Err(NimbleError::from("Session expired"));
        }

        self.profile.validate_transfer(transfer, self.verifying_key.clone().ok_or_else(|| NimbleError::from("Session not unlocked"))?)?;
        
        Ok(())
    }

    pub fn validate_message(&self, msg: Vec<u8>, signature: &[u8; 64]) -> Result<(), NimbleError> {
        if self.is_expired() {
            return Err(NimbleError::from("Session expired"));
        }
        
        self.profile.validate_message(msg, signature, self.verifying_key.clone().ok_or_else(|| NimbleError::from("Session not unlocked"))?)?;

        Ok(())
    }

    pub fn clear_sensitive(&mut self) {
        self.signing_key = None; // Zeroizing garante limpeza
        self.unlocked_at = None;
    }

    /// Limpa os dados da sessão (para segurança)
    pub fn clear(&mut self) {
        self.profile.zeroize();
        // Adicione zeroization no Identity também, se quiser
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{identity::identity::generate, profile::profile_type::ProfileType};

    fn create_test_session() -> Session {
        let seed = [42u8; 32];
        let password = "test-password".to_string();
        let bundle = generate(&seed, password.clone()).unwrap();

        let session = Session::new(bundle.identity).unwrap();
        session
    }

    #[test]
    fn test_session_creation() {
        let session = create_test_session();
        assert_eq!(session.profile.id(), "exposed");
        assert!(session.identity.exposed.is_public());
        assert!(!session.identity.hidden.is_public());
    }
    
    #[test]
    fn test_switch_profile() {
        let mut session = create_test_session();
        session.switch_profile().unwrap();
        assert_eq!(session.profile.id(), "hidden");
        assert_ne!(session.profile.id(), "exposed");
    }

    #[test]
    fn test_session_sign_message() {
        let mut session = create_test_session();
        let message = b"hello nimble";

        session.unlock_key("test-password".to_string()).unwrap();
        let signature = session.sign_message(message)
            .expect("Signing should succeed");

        assert_eq!(signature.len(), 64); // ed25519 signatures são sempre 64 bytes
    }

    #[test]
    fn test_session_clear() {
        let mut session = create_test_session();

        session.clear();

        // Verifica que campos sensíveis foram zerados
        assert!(session.profile.is_cleared(), "Profile should be cleared");
    }
}
