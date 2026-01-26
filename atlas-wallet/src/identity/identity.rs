use ed25519_dalek::{VerifyingKey, SigningKey};
use crate::profile::{exposed::ExposedProfile, hidden::HiddenProfile, profile_type::ProfileType};


use serde::{Serialize, Deserialize};


use super::{
    sk_derivation::derive_dual_profiles, 
    serde_pubkey,
    errors::IdentityError
};

/// Represents a logical user identity composed of multiple profiles.
///
/// An identity can contain multiple profiles (e.g., `hidden`, `exposed`)
/// each with its own keypair, permissions, and visibility settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// List of profiles associated with this identity.
    pub exposed: ExposedProfile,
    pub hidden: HiddenProfile,
}

/// A bundle containing all cryptographic and logical components of an identity.
///
/// Used to encapsulate the master secret key, derived profile keys, and the
/// identity structure itself.
#[derive(Debug, Serialize, Deserialize)]
pub struct IdentityBundle {
    pub sk_master: [u8; 32],
    pub sk_exposed: [u8; 32],

    #[serde(with = "serde_pubkey")]
    pub pub_exposed: VerifyingKey,

    pub sk_hidden: [u8; 32],

    #[serde(with = "serde_pubkey")]
    pub pub_hidden: VerifyingKey,

    pub identity: Identity,
}

impl Identity {
    /// Creates a new empty `Identity`.
    pub fn new(exposed: ExposedProfile, hidden: HiddenProfile) -> Self {
        Identity {
            exposed, 
            hidden,
        }
    }

    /// Prints a list of all profiles and their visibility.
    pub fn list(&self) {
        println!("🔐 Hidden: {}", self.hidden.id());
        println!("🔐 Exposed: {}", self.exposed.id());
        
    }
}

/// Generates a full identity bundle from a cryptographically secure seed.
///
/// This function creates an [`IdentityBundle`] by deriving a master key and two
/// deterministic subkeys (`exposed` and `hidden`) from the given 32-byte seed.
/// Each subkey is used to generate an Ed25519 keypair and create a corresponding
/// [`Profile`] with specific visibility and permissions.
///
/// The generated identity can later be used for signing transactions, managing access,
/// and performing operations based on profile-level permissions.
///
/// # Arguments
///
/// * `seed` - A 32-byte cryptographically secure seed (typically generated via CSPRNG).
///
/// # Returns
///
/// Returns `Ok(IdentityBundle)` on success, or `Err(IdentityError)` if:
/// - Any of the derived keys are invalid (`InvalidSecretKey`);
/// - A profile could not be constructed (`ProfileCreationFailed`);
///
/// # Example
///
/// ```
/// use atlas_wallet::identity::identity::generate;
///
/// let seed = [7u8; 32];
/// let password = "a-strong-password".to_string();
///
/// let identity_bundle = generate(&seed, password).unwrap();
///
/// // You can now access the generated profiles
/// assert_eq!(identity_bundle.identity.exposed.id(), "exposed");
/// assert_eq!(identity_bundle.identity.hidden.id(), "hidden");
/// ```
///
/// # Notes
///
/// This function assumes the seed is strong and securely generated. For password-based
/// identity generation, consider deriving the seed via a KDF like Argon2 before calling this.
pub fn generate(seed: &[u8; 32], password: String) -> Result<IdentityBundle, IdentityError> {
    let (sk_exposed, sk_hidden) = derive_dual_profiles(seed)?;

    let pub_exposed = VerifyingKey::from(
        &SigningKey::from_bytes(&sk_exposed)
    );
    let pub_hidden = VerifyingKey::from(
        &SigningKey::from_bytes(&sk_hidden)
    );

    let profile_exposed = ExposedProfile::new_from_seed(
        &sk_exposed, 
        password.clone(),
        "exposed", 
        vec!["transfer".to_string()]
    )?;

    let profile_hidden  = HiddenProfile::new_from_seed(
        &sk_hidden,
        password,
        "hidden", 
        vec!["stake".to_string()]
    )?;

    let identity = Identity::new(
        profile_exposed,
        profile_hidden
    );

    Ok(IdentityBundle {
        sk_master: *seed,
        sk_exposed,
        pub_exposed,
        sk_hidden,
        pub_hidden,
        identity,
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    fn test_password() -> String {
        "test-password".to_string()
    }

    #[test]
    fn test_identity_generation_and_structure() {
        let seed = [7u8; 32];
        let bundle = generate(&seed, test_password()).expect("Failed to generate identity");

        // Confirma se os campos principais foram gerados
        assert_eq!(bundle.sk_master, seed);
        assert_eq!(bundle.identity.exposed.id(), "exposed");
        assert_eq!(bundle.identity.hidden.id(), "hidden");

        // Confirma acesso aos perfis por id
        let exposed = bundle.identity.exposed;
        let hidden = bundle.identity.hidden;

        assert!(exposed.is_public());
        assert!(!hidden.is_public());

        assert!(exposed.try_validate_permissions("transfer"));
        assert!(hidden.try_validate_permissions("stake"));
    }


    #[test]
    fn test_public_key_consistency() {
        let seed = [11u8; 32];
        let bundle = generate(&seed, test_password()).unwrap();
        
        let exposed_pk = bundle.identity.exposed;
        let expected_pk = bundle.pub_exposed;

        println!("🔐 Exposed PK: {:?}", exposed_pk.address());
        println!("🔐 Expected PK: {:?}", expected_pk);

        assert_eq!(exposed_pk.data.public_key, expected_pk);
    }
}
