use ed25519_dalek::SignatureError;
use thiserror::Error;
use sha2::digest::InvalidLength;
use argon2::password_hash::Error as Argon2Error;
use atlas_common::address::errors::AddressError;

/// Represents possible errors that can occur during identity generation, profile creation,
/// or key derivation within the Nimble identity system.
#[derive(Debug, Error)]
pub enum IdentityError {
    /// Failed to create a profile (typically due to an invalid secret key or seed).
    ///
    /// Wraps a `SignatureError` from `ed25519_dalek`, which can occur
    /// if the seed is not valid for key generation.
    #[error("Failed to create profile: {0}")]
    ProfileCreationFailed(SignatureError),

    /// Indicates that a provided secret key (usually from bytes) is invalid.
    ///
    /// This is returned when calling `SigningKey::from_bytes(...)` fails,
    /// typically due to a malformed or incorrect key length.
    #[error("Invalid secret key: {0}")]
    InvalidSecretKey(SignatureError),

    /// Returned when a profile is requested by ID but is not found in the identity.
    ///
    /// Contains the requested profile ID for reference and debugging.
    #[error("Profile not found: {0}")]
    ProfileNotFound(String),

    /// The provided salt could not be encoded to a valid Base64 string for use in Argon2.
    ///
    /// This usually indicates that the salt is malformed or does not meet Argon2's encoding expectations.
    #[error("Invalid salt: could not encode as base64")]
    InvalidSaltBase64,

    /// Argon2 password hashing failed internally.
    ///
    /// This might occur due to misconfigured parameters, missing dependencies,
    /// or internal errors in the hashing implementation.
    #[error("Argon2 password hashing failed: {0}")]
    Argon2HashingFailed(String),

    /// The HMAC key used during key derivation is invalid.
    ///
    /// This error occurs when the provided key length does not meet the expected size
    /// (e.g., not 32 bytes when expected by the HMAC implementation).
    #[error("Invalid HMAC key size")]
    InvalidHmacKey(hmac::digest::InvalidLength),

    /// An error occurred while converting a public key to a bech32m address.
    ///
    /// This error wraps issues related to byte conversion or encoding within address formatting.
    #[error("Address error: {0}")]
    AddressError(#[from] AddressError),

    /// Public key length is not 32 bytes.
    #[error("Invalid public key length: {0}")]
    InvalidPublicKeyLength(usize),

    /// Public key is not valid.
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    /// Private key is not valid.
    #[error("Invalid private key: {0}")]
    InvalidPrivateKey(String),

    /// The Argon2 result did not contain a hash output.
    ///
    /// This typically means that the hash was not properly computed or returned empty.
    #[error("Missing hash output after Argon2 processing")]
    MissingHash,

    /// Failed to encrypt data.
    #[error("Failed to encrypt data with password: {0}")]
    EncryptionFailed(String),

    /// Failed to decrypt data.
    #[error("Failed to decrypt data: {0}")]
    DecryptionFailed(String),

    /// Failed Deserialization of data.
    #[error("Failed to deserialize data: {0}")]
    DeserializationFailed(String),

    /// Failed Vault Deserialization of data.
    #[error("Failed to deserialize vault: {0}")]
    VaultDeserializationFailed(String),

    /// Failed to encode/decode base58.
    #[error("Failed to encode/decode base58: {0}")]
    FromBase58(#[from] bs58::decode::Error),

    /// The provided permission is invalid.
    #[error("Invalid permission: {0}")]
    InvalidPermission(String),

    /// File already exists.
    #[error("File already exists: {0}")]
    FileAlreadyExists(String),
}


impl From<InvalidLength> for IdentityError {
    fn from(err: InvalidLength) -> Self {
        IdentityError::InvalidHmacKey(err)
    }
}

impl From<Argon2Error> for IdentityError {
    fn from(err: Argon2Error) -> Self {
        IdentityError::Argon2HashingFailed(err.to_string())
    }
}

impl From<argon2::Error> for IdentityError {
    fn from(err: argon2::Error) -> Self {
        IdentityError::Argon2HashingFailed(err.to_string())
    }
}