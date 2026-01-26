use serde::{Serialize, Deserialize};
use bincode;
use aes_gcm::{Aes256Gcm, KeyInit, aead::{Aead, generic_array::GenericArray}, Nonce};
use std::fs;
use zeroize::{Zeroize, ZeroizeOnDrop};
use atlas_common::utils::security::{
    generate_salt, 
    generate_nonce
};
use crate::{
    errors::NimbleError, 
    identity::{
        errors::IdentityError, 
        identity::IdentityBundle
    }, 
    session::session::Session, 
    vault::password::Password
};

#[derive(Debug, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct VaultData {
    pub version: u8,
    pub ciphertext: Vec<u8>,
    pub nonce: [u8; 12],
    salt: [u8; 16],
    iterations: u32,
    memory: u32,
    parallelism: u32,
    context: String,
}

impl VaultData {

    pub fn new(version: u8, ciphertext: Vec<u8>) -> Self {
        Self {
            version,
            ciphertext,
            nonce: generate_nonce(),
            salt: generate_salt(),
            iterations: Self::generate_iterations(),
            memory: Self::generate_memory(),
            parallelism: Self::generate_parallelism(),  
            context: "vault".to_string(),
        }
    }

    fn prepare_password(&self, password: String, salt: [u8; 16], iterations: u32, memory: u32, parallelism: u32) -> Password {
        Password::new(password.into_bytes(), salt, iterations, memory, parallelism)
    }

    pub fn load_session(
        &self,
        password: String,
        path: &str,
    ) -> Result<Session, NimbleError> {
        let encoded = fs::read(path).map_err(|e| IdentityError::DecryptionFailed(e.to_string()))?;
        let bundle = self.load_identity_bundle(password, encoded)?;
        let session = Session::from_bundle(bundle)?;
        Ok(session)
    }

    pub fn load_identity_bundle(
        &self,
        password: String,
        encoded: Vec<u8>,
    ) -> Result<IdentityBundle, IdentityError> {
        let vault: VaultData = bincode::deserialize(&encoded)
            .map_err(|e| IdentityError::VaultDeserializationFailed(e.to_string()))?;

        let mut password = self.prepare_password(password, vault.salt, vault.iterations, vault.memory, vault.parallelism);
        let cipher = Aes256Gcm::new(GenericArray::from_slice(&password.derive_secret(&self.context)?));
        password.zeroize();

        let decrypted = cipher.decrypt(GenericArray::from_slice(&vault.nonce), vault.ciphertext.as_ref())
            .map_err(|e| IdentityError::DecryptionFailed(e.to_string()))?;

        let bundle: IdentityBundle = bincode::deserialize(&decrypted).map_err(|e| IdentityError::DeserializationFailed(e.to_string()))?;

        Ok(bundle)
    }

    pub fn save_identity_bundle(
        &self,
        bundle: &IdentityBundle,
        password: String,
    ) -> Result<Vec<u8>, IdentityError> {
        //self.ensure_path_does_not_exist(path)?;

        let serialized = bincode::serialize(bundle)
            .map_err(|e | IdentityError::EncryptionFailed(e.to_string()))?;

        let nonce_bytes = generate_nonce();

        let mut password: Password = self.prepare_password(password, self.salt, self.iterations, self.memory, self.parallelism);

        let cipher = Aes256Gcm::new(GenericArray::from_slice(&password.derive_secret(&self.context)?));
        password.zeroize();

        let nonce = Nonce::from_slice(&nonce_bytes);
        let encrypted = cipher.encrypt(nonce, serialized.as_ref())
            .map_err(|e | IdentityError::EncryptionFailed(e.to_string()))?;

        let vault = VaultData {
            version: 1,
            ciphertext: encrypted,
            nonce: nonce_bytes,
            salt: self.salt,
            iterations: self.iterations,
            memory: self.memory,
            parallelism: self.parallelism,
            context: self.context.clone(),
        };

        let encoded = bincode::serialize(&vault)
            .map_err(|e | IdentityError::EncryptionFailed(e.to_string()))?;

        //fs::write(path, encoded).map_err(|e | IdentityError::EncryptionFailed(e.to_string()))?;
        
        Ok(encoded)
    }

    fn generate_iterations() -> u32 {
        3 // valor padrão seguro para Argon2id
    }

    fn generate_memory() -> u32 {
        65536 // 64 MiB
    }

    fn generate_parallelism() -> u32 {
        1 // geralmente 1 é seguro, mas pode ser maior se quiser usar várias threads
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::identity::{IdentityBundle, generate};
    use atlas_common::utils::security::generate_seed;
    use atlas_common::address::profile_address::ProfileAddress;
    use std::fs;

    fn create_dummy_identity_bundle() -> IdentityBundle {
        let seed = generate_seed();
        let bundle = generate(&seed, "test_password".into()).unwrap();
        bundle
    }

    #[test]
    fn test_save_and_load_identity_bundle() {
        let password = "test_password".to_string();
        let original_bundle = create_dummy_identity_bundle();
        
        let vault = VaultData::new(1, vec![]);

        // Salvar o bundle em memória
        let encoded_data = vault.save_identity_bundle(&original_bundle, password.clone())
            .expect("Failed to save bundle");

        // Carregar o bundle a partir dos dados em memória
        let loaded_bundle = vault.load_identity_bundle(password.clone(), encoded_data)
            .expect("Failed to load bundle");

        // Comparar os endereços dos perfis para garantir que a desserialização foi correta
        assert_eq!(original_bundle.identity.hidden.address().as_str(), loaded_bundle.identity.hidden.address().as_str());
        assert_eq!(original_bundle.identity.exposed.address().as_str(), loaded_bundle.identity.exposed.address().as_str());
        
        // Comparar as chaves públicas para ter uma verificação mais forte
        assert_eq!(original_bundle.pub_exposed, loaded_bundle.pub_exposed);
        assert_eq!(original_bundle.pub_hidden, loaded_bundle.pub_hidden);
    }
}
