use std::collections::HashMap;
use super::{error::KycError, model::{KycLevel, KycNft}};

/// In-memory registry of symbolic KYC NFTs, indexed by `subject` (profile ID).
///
/// The `KycRegistry` is responsible for storing and managing the lifecycle of KYC NFTs.
/// It supports registration, revocation, querying by subject, and listing of active KYC entries.
///
/// Typically used internally by a [`KycService`] layer for business rules.
#[derive(Debug, Default, Clone)]
pub struct KycRegistry {
    store: HashMap<String, KycNft>,
}

impl KycRegistry {
    /// Creates a new, empty KYC registry.
    ///
    /// # Example
    /// ```
    /// use nimble_protocol::core::nft_kyc::registry::KycRegistry;
    /// 
    /// let registry = KycRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self { store: HashMap::new() }
    }

    /// Registers a new KYC NFT for a given subject.
    ///
    /// # Errors
    /// Returns [`KycError::KycAlreadyExists`] if the subject already has an active (non-revoked) KYC entry.
    ///
    /// # Example
    /// ```
    /// use nimble_protocol::core::nft_kyc::registry::KycRegistry;
    /// 
    /// let nft = KycNft::new("user123", "issuer", KycLevel::Basic, current_time(), None, None);
    /// registry.register(nft)?;
    /// ```
    pub fn register(&mut self, nft: KycNft) -> Result<(), KycError> {
        if self.has_valid(&nft.subject) {
            return Err(KycError::KycAlreadyExists(nft.subject.clone()));
        }

        self.store.insert(nft.subject.clone(), nft);
        Ok(())
    }

    /// Revokes the active KYC NFT of a subject, if it exists.
    ///
    /// # Errors
    /// - [`KycError::KycNotFound`] if the subject has no KYC registered.
    /// - [`KycError::KycInactive`] if the KYC exists but is already revoked.
    pub fn revoke(&mut self, subject_id: &str) -> Result<bool, KycError> {
        if !self.exists(subject_id) {
            return Err(KycError::KycNotFound(subject_id.to_string()));
        }

        let nft = self.store
            .get_mut(subject_id)
            .ok_or_else(|| KycError::KycNotFound(subject_id.to_string()))?;

        if !nft.is_active() {
            return Err(KycError::KycInactive(subject_id.to_string()));
        }
        nft.revoke();
        Ok(true)
    }

    /// Retrieves the KYC NFT of a subject, **only if it is active**.
    ///
    /// # Returns
    /// - `Ok(&KycNft)` if the subject has an active KYC.
    /// - `Err(KycError::KycNotFound)` otherwise.
    pub fn get(&self, subject_id: &str) -> Result<&KycNft, KycError> {
        match self.store.get(subject_id) {
            Some(nft) if nft.is_active() => Ok(nft),
            _ => Err(KycError::KycNotFound(subject_id.to_string())),
        }
    }

    /// Retrieves the KYC NFT of a subject, even if it has been revoked.
    ///
    /// Useful for audit or administrative purposes.
    pub fn get_any(&self, subject_id: &str) -> Result<&KycNft, KycError> {
        self.store
            .get(subject_id)
            .ok_or_else(|| KycError::KycNotFound(subject_id.to_string()))
    }

    /// Retrieves a mutable reference to a subject’s KYC NFT, if any.
    ///
    /// # Returns
    /// `Some(&mut KycNft)` if a KYC exists, regardless of active status.
    pub fn get_mut(&mut self, subject_id: &str) -> Option<&mut KycNft> {
        self.store.get_mut(subject_id)
    }

    /// Checks if the subject has a valid (non-revoked) KYC NFT.
    ///
    /// # Returns
    /// `true` if an active KYC exists for the subject, `false` otherwise.
    pub fn has_valid(&self, subject_id: &str) -> bool {
        self.store
            .get(subject_id)
            .map(|nft| nft.is_active())
            .unwrap_or(false)
    }

    /// Returns the current KYC level of the subject, **if active**.
    ///
    /// # Returns
    /// - `Some(KycLevel)` if an active KYC exists.
    /// - `None` otherwise.
    pub fn level_of(&self, subject_id: &str) -> Option<KycLevel> {
        self.store
            .get(subject_id)
            .filter(|nft| nft.is_active())
            .map(|nft| nft.level)
    }

    /// Checks whether a subject has **any** KYC NFT, regardless of its revocation status.
    ///
    /// # Returns
    /// `true` if the subject has a KYC NFT (revoked or not), `false` otherwise.
    pub fn exists(&self, subject_id: &str) -> bool {
        self.store.contains_key(subject_id)
    }

    /// Lists all subject IDs that currently have an active KYC NFT.
    ///
    /// # Returns
    /// A vector of subject ID strings.
    pub fn list_active_subjects(&self) -> Vec<&str> {
        self.store
            .iter()
            .filter(|(_, nft)| nft.is_active())
            .map(|(id, _)| id.as_str())
            .collect()
    }
}
