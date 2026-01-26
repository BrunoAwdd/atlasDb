use super::error::KycError;
use super::model::{KycNft, KycLevel};
use super::registry::KycRegistry;

/// Service responsible for issuing, managing, and validating KYC NFTs.
///
/// The `KycService` acts as a high-level interface to manage the KYC lifecycle of user profiles,
/// including issuance, revocation, upgrading, and permission checks based on verification levels.
/// It operates on top of a [`KycRegistry`], which holds the actual storage of KYC NFTs.
///
/// This service ensures:
/// - Only one active KYC per profile (`subject`) is allowed
/// - KYC levels are enforced consistently
/// - Revocation and upgrades are straightforward
pub struct KycService<'a> {
    /// Mutable reference to the NFT registry that stores all KYC entries.
    pub registry: &'a mut KycRegistry,
}

impl<'a> KycService<'a> {
    /// Issues and registers a new KYC NFT for a given profile.
    ///
    /// Fails if the subject already has a valid (non-revoked) KYC entry.
    ///
    /// # Arguments
    /// * `subject` - The profile ID receiving the KYC.
    /// * `issuer` - The entity issuing the KYC (e.g., authority or institution).
    /// * `level` - The KYC level to assign (e.g., `Basic`, `Advanced`).
    /// * `issued_at` - Timestamp of issuance (usually `current_time()`).
    /// * `metadata` - Optional metadata (e.g., document hash).
    /// * `external_url` - Optional link to an external verification portal.
    ///
    /// # Errors
    /// Returns an error if the profile already has an active KYC.
    pub fn emit_kyc(
        &mut self,
        subject: &str,
        issuer: &str,
        level: KycLevel,
        issued_at: u64,
        metadata: Option<String>,
        external_url: Option<String>,
    ) -> Result<(), KycError> {
        if self.registry.has_valid(subject) {
            return Err(KycError::KycAlreadyExists(subject.to_string()).into());
        }

        let nft = KycNft::new(subject, issuer, level, issued_at, metadata, external_url);
        self.registry.register(nft)?;
        Ok(())
    }

    /// Revokes the active KYC NFT of the given subject, if it exists.
    ///
    /// # Arguments
    /// * `subject` - The profile ID whose KYC should be revoked.
    ///
    /// # Errors
    /// Returns an error if no active KYC was found.
    pub fn revoke_kyc(&mut self, subject: &str) -> Result<(), KycError> {
        if self.registry.revoke(subject)? {
            Ok(())
        } else {
            Err(KycError::KycNotFound(subject.to_string()).into())
        }
    }

    /// Retrieves the current verification level of a profile.
    ///
    /// # Arguments
    /// * `subject` - The profile ID to query.
    ///
    /// # Returns
    /// `Some(KycLevel)` if a valid KYC exists, or `None` otherwise.
    pub fn get_level(&self, subject: &str) -> Option<KycLevel> {
        self.registry.level_of(subject)
    }

    /// Checks if a profile satisfies a required minimum KYC level.
    ///
    /// # Arguments
    /// * `subject` - The profile ID to validate.
    /// * `required` - The minimum `KycLevel` needed for the operation.
    ///
    /// # Returns
    /// `true` if the current level is equal to or higher than required.
    /// Returns `false` if the profile has no valid KYC or insufficient level.
    pub fn satisfies(&self, subject: &str, required: KycLevel) -> bool {
        match self.get_level(subject) {
            Some(current) => current >= required,
            None => false,
        }
    }

    /// Attempts to upgrade the KYC level of a profile.
    ///
    /// Only upgrades if the new level is strictly greater than the current one.
    /// Automatically resets the `revoked` flag to `false` if applicable.
    ///
    /// # Arguments
    /// * `subject` - The profile ID to upgrade.
    /// * `new_level` - The new desired level.
    /// * `new_issued_at` - The timestamp of the upgrade.
    ///
    /// # Errors
    /// Returns an error if the profile has no registered KYC.
    pub fn upgrade_kyc(&mut self, subject: &str, new_level: KycLevel, new_issued_at: u64) -> Result<(), KycError> {
        if let Some(nft) = self.registry.get_mut(subject) {
            nft.upgrade(new_level, new_issued_at);
            Ok(())
        } else {
            Err(KycError::KycNotFound(subject.to_string()).into())
        }
    }

    /// Checks whether the given subject has an active (non-revoked) KYC.
    ///
    /// # Arguments
    /// * `subject` - The profile ID to check.
    ///
    /// # Returns
    /// `true` if a valid KYC exists, `false` otherwise.
    pub fn is_active(&self, subject: &str) -> bool {
        self.registry.has_valid(subject)
    }
}
