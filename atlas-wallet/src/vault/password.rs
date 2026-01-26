
use argon2::{Argon2, Params};
use zeroize::{Zeroize, ZeroizeOnDrop};
use sha2::Digest;

use crate::identity::errors::IdentityError;

#[derive(Zeroize, ZeroizeOnDrop, Clone)]
pub struct Password {
    password: Vec<u8>,
    salt: [u8; 16],
    iterations: u32,
    memory: u32,
    parallelism: u32,
}


impl Password {
    pub fn new(password: Vec<u8>, salt: [u8; 16], iterations: u32, memory: u32, parallelism: u32) -> Self {
        Self {
            password,
            salt,
            iterations,
            memory,
            parallelism,
        }
    }
    pub fn derive_secret(&self, context: &str) -> Result<[u8; 32], IdentityError> {
        // Deriva um novo salt usando o salt original + context
        let mut hasher = sha2::Sha256::new();
        hasher.update(&self.salt);
        hasher.update(context.as_bytes());
        let derived_salt = hasher.finalize();

        let params = Params::new(
            self.memory,
            self.iterations,
            self.parallelism,
            Some(32),
        ).map_err(|_| IdentityError::EncryptionFailed("Invalid Argon2 parameters".to_string()))?;

        let argon2 = Argon2::new(
            argon2::Algorithm::Argon2id,
            argon2::Version::V0x13,
            params,
        );

        let mut secret = [0u8; 32];
        argon2.hash_password_into(
            &self.password,
            &derived_salt[..16], // usa só os primeiros 16 bytes como salt final
            &mut secret,
        ).map_err(|_| IdentityError::EncryptionFailed("Argon2 hashing failed".to_string()))?;

        Ok(secret)
    }
    

}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_common::utils::security::generate_salt;

    #[test]
    fn test_password_new() {
        let salt = generate_salt();

        let password1 = Password::new(b"secret".to_vec(), salt, 3, 65536, 1);
        let secret1 = password1.derive_secret("test").unwrap();
        let password2 = Password::new(b"secret".to_vec(), salt, 3, 65536, 1);
        let secret2 = password2.derive_secret("test").unwrap();
        
        assert_eq!(secret1, secret2);
    }

    #[test]
    fn test_password_with_different_password() {
        let salt = generate_salt();

        let password1 = Password::new(b"secret".to_vec(), salt, 3, 65536, 1);
        let secret1 = password1.derive_secret("test").unwrap();
        let password2 = Password::new(b"secret2".to_vec(), salt, 3, 65536, 1);
        let secret2 = password2.derive_secret("test").unwrap();
        
        assert_ne!(secret1, secret2);
    }

    #[test]
    fn test_password_with_different_salt() {
        let password1 = Password::new(b"secret".to_vec(), generate_salt(), 3, 65536, 1);
        let secret1 = password1.derive_secret("test").unwrap();
        let password2 = Password::new(b"secret".to_vec(), generate_salt(), 3, 65536, 1);
        let secret2 = password2.derive_secret("test").unwrap();
        
        assert_ne!(secret1, secret2);
    }

    #[test]
    fn test_password_with_different_iterations() {
        let salt = generate_salt();

        let password1 = Password::new(b"secret".to_vec(), salt, 3, 65536, 1);
        let secret1 = password1.derive_secret("test").unwrap();
        let password2 = Password::new(b"secret".to_vec(), salt, 2, 65536, 1);
        let secret2 = password2.derive_secret("test").unwrap();
        
        assert_ne!(secret1,secret2);
    }

    #[test]
    fn test_password_with_different_memory() {
        let salt = generate_salt();

        let password1 = Password::new(b"secret".to_vec(), salt, 3, 65536, 1);
        let secret1 = password1.derive_secret("test").unwrap();
        let password2 = Password::new(b"secret".to_vec(), salt, 3, 32768, 1);
        let secret2 = password2.derive_secret("test").unwrap();
        
        assert_ne!(secret1,secret2);
    }


    #[test]
    fn test_password_with_different_parallelism() {
        let salt = generate_salt();

        let password1 = Password::new(b"secret".to_vec(), salt, 3, 65536, 1);
        let secret1 = password1.derive_secret("test").unwrap();
        let password2 = Password::new(b"secret".to_vec(), salt, 3, 65536, 2);
        let secret2 = password2.derive_secret("test").unwrap();
        
        assert_ne!(secret1,secret2);
    }

    #[test]
    fn test_password_with_different_context() {
        let salt = generate_salt();

        let password1 = Password::new(b"secret".to_vec(), salt, 3, 65536, 1);
        let secret1 = password1.derive_secret("test").unwrap();
        let password2 = Password::new(b"secret".to_vec(), salt, 3, 65536, 1);
        let secret2 = password2.derive_secret("test2").unwrap();
        
        assert_ne!(secret1,secret2);
    }

    #[test]
    fn test_password_with_invalid_salt() {
        let password = Password::new(b"secret".to_vec(), [0u8; 16], 3, 65536, 1);
        let secret = password.derive_secret("test").unwrap();
        assert_eq!(secret.len(), 32);
    }

    #[test]
    fn test_password_with_invalid_iterations() {
        let salt = generate_salt();

        let password = Password::new(b"secret".to_vec(), salt, 0, 65536, 1);
        let result = password.derive_secret("test");

        assert!(result.is_err());
    }

    #[test]
    fn test_password_with_invalid_memory() {
        let salt = generate_salt();

        let password = Password::new(b"secret".to_vec(), salt, 3, 0, 1);
        let result = password.derive_secret("test");
        assert!(result.is_err());
    }

    #[test]
    fn test_password_with_invalid_parallelism() {
        let salt = generate_salt();

        let password = Password::new(b"secret".to_vec(), salt, 3, 65536, 0);
        let result = password.derive_secret("test");
        assert!(result.is_err());
    }   

}
