use aes_gcm::{
    Aes256Gcm, 
    aead::{Aead, KeyInit, generic_array::GenericArray}
};
use ed25519_dalek::SigningKey;

use crate::vault::password::Password;
use super::errors::IdentityError;

use atlas_common::utils::security::generate_nonce;

fn derive_password_secret(password: String, salt: [u8; 16], context: &str) -> Result<[u8; 32], IdentityError> {
    let password = password.into_bytes();
    let iterations = 3;
    let memory = 65536;
    let parallelism = 1;


    let pass = Password::new(
        password,
        salt,
        iterations,
        memory,
        parallelism,
    );
    
    pass.derive_secret(context)
}

pub fn encrypt_secret_key(sk: &SigningKey, password: String, salt: [u8; 16]) -> Result<String, IdentityError> {
    let key = derive_password_secret(password, salt, "profile:sk")?;
    let encrypted = encrypt_data(&sk.to_bytes(), &key)?;
    Ok(bs58::encode(encrypted).into_string())
}

pub fn decrypt_secret_key(encoded: &str, password: String, salt: [u8; 16]) -> Result<SigningKey, IdentityError> {
    let key = derive_password_secret(password, salt,"profile:sk")?;
    let decoded = bs58::decode(&encoded).into_vec()?;

    let decrypted = decrypt_data(&decoded, &key)?;
    let secret_key = SigningKey::from_bytes(&decrypted);
    Ok(secret_key)
}

fn encrypt_data(data: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, IdentityError> {
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));

    let nonce = generate_nonce();

    let ciphertext = cipher.encrypt(GenericArray::from_slice(&nonce), data)
        .map_err(|e | IdentityError::EncryptionFailed(e.to_string()))?;
    let mut output = nonce.to_vec();
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

fn decrypt_data(encrypted: &[u8], key: &[u8; 32]) -> Result<[u8; 32], IdentityError> {
    if encrypted.len() < 12 {
        return Err(IdentityError::DecryptionFailed("Invalid encrypted data: too short".to_string()));
    }

    let (nonce, ciphertext) = encrypted.split_at(12);
    let cipher = Aes256Gcm::new(GenericArray::from_slice(key));
    let decrypted = cipher.decrypt(GenericArray::from_slice(nonce), ciphertext)
        .map_err(|e| IdentityError::DecryptionFailed(e.to_string()))?;

    
    let array: [u8; 32] = decrypted
        .try_into()
        .map_err(|_| IdentityError::DecryptionFailed("Decrypted data is not 64 bytes".to_string()))?;

    Ok(array)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;
    use crate::identity::errors::IdentityError;
    use atlas_common::utils::security::{generate_salt, generate_seed};

    /// Testa se uma chave secreta pode ser criptografada e depois corretamente descriptografada com a mesma senha.
    #[test]
    fn test_encrypt_and_decrypt_secret_key() -> Result<(), IdentityError> {
        let salt = generate_salt();
        let seed = generate_seed();

        let sk = SigningKey::from_bytes(&seed);

        let password = "my-secure-password".to_string();
        let encrypted = encrypt_secret_key(&sk, password.clone(), salt)?;

        let decrypted = decrypt_secret_key(&encrypted, password, salt)?;

        assert_eq!(sk.to_bytes(), decrypted.to_bytes());

        Ok(())
    }

    /// Testa se descriptografar com a senha errada falha.
    #[test]
    fn test_decrypt_with_wrong_password_fails() {
        let salt = generate_salt();
        let seed = generate_seed();

        let sk = SigningKey::from_bytes(&seed);

        let encrypted = encrypt_secret_key(&sk, "correct-password".to_string(), salt).unwrap();

        let result = decrypt_secret_key(&encrypted, "wrong-password".to_string(), salt);

        assert!(result.is_err());
    }

    /// Testa se descriptografar dados corrompidos falha.
    #[test]
    fn test_decrypt_invalid_data_fails() {
        let salt = generate_salt();
        let invalid_data = "invalid_base58_string!";

        let result = decrypt_secret_key(invalid_data, "any-password".to_string(), salt);

        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_with_wrong_salt_fails() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();
        let seed = generate_seed();

        let sk = SigningKey::from_bytes(&seed);

        let encrypted = encrypt_secret_key(&sk, "my-password".to_string(), salt1).unwrap();
        let result = decrypt_secret_key(&encrypted, "my-password".to_string(), salt2);

        assert!(result.is_err(), "Decryption should fail with wrong salt");
    }

}
