use thiserror::Error;

/// Defines errors related to KYC NFT operations and validation.
#[derive(Debug, Error)]
pub enum KycError {
    /// The subject already has an active (non-revoked) KYC NFT.
    #[error("Profile '{0}' already has a KYC NFT.")]
    KycAlreadyExists(String),

    /// No KYC NFT was found for the subject.
    #[error("Profile '{0}' has no KYC NFT.")]
    KycNotFound(String),

    /// The KYC NFT exists but is inactive (revoked).
    #[error("Profile '{0}' has an inactive KYC NFT.")]
    KycInactive(String),

    /// The KYC NFT is unexpectedly active (used when revocation or override was expected).
    #[error("Profile '{0}' has an active KYC NFT.")]
    KycActive(String),

    /// The KYC NFT exists but has a revoked status (semantically stronger than inactive).
    #[error("Profile '{0}' has a revoked KYC NFT.")]
    KycRevoked(String),

    /// The KYC NFT has an invalid or unacceptable level (e.g., below minimum required).
    #[error("Profile '{0}' has a KYC NFT with an invalid level.")]
    KycInvalidLevel(String),

    /// The issued timestamp on the KYC NFT is invalid (e.g., in the future or too old).
    #[error("Profile '{0}' has a KYC NFT with an invalid issued timestamp.")]
    KycInvalidTimestamp(String),

    // Future expansion point:
    // #[error("Profile '{0}' is not authorized to perform this KYC operation.")]
    // UnauthorizedOperation(String),
}
