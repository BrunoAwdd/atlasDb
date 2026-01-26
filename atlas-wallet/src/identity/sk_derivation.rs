
use ed25519_dalek::SigningKey;
use sha2::Sha256;
use hmac::{Hmac, Mac};
use crate::identity::errors::IdentityError;

type HmacSha256 = Hmac<Sha256>;

/// Converts a 32-byte secure seed into an Ed25519 secret key (`SigningKey`).
///
/// This function interprets the provided seed as the raw bytes for an Ed25519 private key.
/// It is deterministic and context-free: the same seed will always produce the same `SigningKey`.
///
/// # Arguments
///
/// * `seed` - A 32-byte cryptographically secure seed (e.g., generated via CSPRNG).
///
/// # Errors
///
/// Returns an `IdentityError::InvalidPrivateKey` if the seed is not a valid Ed25519 secret key.
/// (In practice, all 32-byte inputs are acceptable unless additional constraints are applied.)
///
/// # Returns
///
/// An Ed25519 `SigningKey` derived directly from the seed.
///
/// # Example
///
/// ```
/// use ed25519_dalek::SigningKey;
/// use atlas_wallet::identity::sk_derivation::derive_sk_from_seed;
/// let seed = [42u8; 32];
/// let secret_key = derive_sk_from_seed(&seed);
/// ```
pub fn derive_sk_from_seed(
    seed: &[u8; 32],
) -> SigningKey {
    SigningKey::from_bytes(seed)
}

/// Derives two subkeys (`exposed`, `hidden`) from a given master key.
///
/// This function uses the [`derive_subkey`] function twice with distinct contexts
/// to produce two deterministic, domain-separated subkeys.
///
/// # Arguments
///
/// * `sk_master` - The 32-byte master secret key derived from user credentials.
///
/// # Returns
///
/// A tuple containing:
/// - A derived key for the `"exposed"` context
/// - A derived key for the `"hidden"` context
///
/// # Example
///
/// ```
/// use atlas_wallet::identity::sk_derivation::derive_dual_profiles;
/// let master = [1u8; 32];
/// let (k1, k2) = derive_dual_profiles(&master).unwrap();
/// assert_ne!(k1, k2); // Different contexts produce different keys
/// ```
pub fn derive_dual_profiles(sk_master: &[u8; 32]) -> Result<([u8; 32], [u8; 32]), IdentityError> {
    let sk_exposed = derive_subkey(sk_master, "exposed")?;
    let sk_hidden = derive_subkey(sk_master, "hidden")?;
    Ok((sk_exposed, sk_hidden))
}

/// Derives a context-specific subkey from a master seed using HMAC-SHA256.
///
/// This function takes a secure 32-byte seed and produces a domain-separated subkey
/// by applying HMAC with a given context string (e.g., `"exposed"`, `"hidden"`, etc).
/// It is suitable for creating deterministic but isolated key derivations.
///
/// # Arguments
///
/// * `seed` - A 32-byte master seed, typically generated from a secure random source.
/// * `context` - A domain string used to isolate subkeys (e.g. `"signing"`, `"auth"`, `"encryption"`).
///
/// # Returns
///
/// A 32-byte derived subkey as `[u8; 32]`.
///
/// # Errors
///
/// Returns `IdentityError::InvalidHmacKey` if the seed is not suitable for HMAC (shouldn't happen with 32 bytes).
///
fn derive_subkey(seed: &[u8; 32], context: &str) -> Result<[u8; 32], IdentityError> {
    let mut mac = HmacSha256::new_from_slice(seed)
        .map_err(IdentityError::InvalidHmacKey)?;
    mac.update(context.as_bytes());
    let result = mac.finalize().into_bytes();

    let mut out = [0u8; 32];
    out.copy_from_slice(&result[..32]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SECRET_KEY_LENGTH;

    #[test]
    fn test_derive_sk_from_seed_valid() {
        let seed = [42u8; 32];
        let sk = derive_sk_from_seed(&seed);
        assert_eq!(sk.as_bytes().len(), SECRET_KEY_LENGTH);
    }

    #[test]
    fn test_derive_dual_profiles_are_different() {
        let seed = [1u8; 32];
        let (exposed, hidden) = derive_dual_profiles(&seed).expect("Failed to derive dual profiles");
        assert_ne!(exposed, hidden, "Derived exposed and hidden keys should be different");
    }

    #[test]
    fn test_derive_subkey_consistency() {
        let seed = [7u8; 32];
        let k1 = derive_subkey(&seed, "context").expect("Failed to derive subkey");
        let k2 = derive_subkey(&seed, "context").expect("Failed to derive subkey");
        assert_eq!(k1, k2, "Subkey derivation should be deterministic for same seed/context");
    }

    #[test]
    fn test_derive_subkey_different_contexts() {
        let seed = [99u8; 32];
        let k1 = derive_subkey(&seed, "exposed").expect("Failed to derive subkey exposed");
        let k2 = derive_subkey(&seed, "hidden").expect("Failed to derive subkey hidden");
        assert_ne!(k1, k2, "Different contexts should produce different subkeys");
    }
}
