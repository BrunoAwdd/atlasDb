use ed25519_dalek::SignatureError;
use thiserror::Error;
use bech32::Error as Bech32Error;

/// Errors related specifically to address formatting and encoding.
#[derive(Debug, Error)]
pub enum AddressError {
    /// Failed to convert bytes into base32 segments (5-bit values).
    ///
    /// Usually caused by input data not being properly aligned or padded for Bech32m encoding.
    #[error("Failed to convert public key bytes: {0}")]
    BitConversionFailed(#[from] Bech32Error),

    /// Failed to encode the address as Bech32m.
    ///
    /// This could happen if the input is invalid or doesn't conform to Bech32m standards.
    #[error("Failed to encode address to bech32m")]
    EncodingFailed,

    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    /// Public key length is not 32 bytes.
    #[error("Invalid public key length: {0}")]
    InvalidPublicKeyLength(usize),
}

impl From<SignatureError> for AddressError {
    fn from(err: SignatureError) -> Self {
        AddressError::InvalidPublicKey(err.to_string())
    }
}