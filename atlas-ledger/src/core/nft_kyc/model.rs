use std::fmt;
use serde::{Serialize, Deserialize};
use crate::core::nft_kyc::proof::calculate_proof_hash;

/// Represents a symbolic NFT used for identity verification within the Kyc on-chain network.
///
/// A `KycNft` acts as a non-transferable credential that reflects a user's verification level.
/// It is typically issued by an authorized entity (such as `"atlas-protocol/bank"`) during
/// onboarding or KYC/KYB processes.
///
/// This NFT can be:
/// - **Upgraded** to a higher verification level (e.g., from `Basic` to `Advanced`)
/// - **Revoked** when the verification is no longer valid
/// - **Queried** to verify if it's still active (`is_active()`)
///
/// # Fields
///
/// - `subject`: The unique identifier of the verified profile (usually `profile.id`)
/// - `issuer`: The entity that issued this NFT (e.g., bank, DAO, validator node)
/// - `level`: The level of verification granted
/// - `issued_at`: The timestamp (in seconds) when the NFT was issued
/// - `revoked`: Indicates whether the NFT is currently active or revoked
/// - `metadata`: Optional extra data (e.g., document hash, DID reference)
/// - `external_url`: Optional link to external metadata or verification portal
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KycNft {
    pub subject: String,
    pub issuer: String,
    pub level: KycLevel,
    pub issued_at: u64,
    pub revoked: bool,
    pub metadata: Option<String>,
    pub external_url: Option<String>,
}

impl KycNft {
    /// Creates a new active (non-revoked) KYC NFT.
    ///
    /// # Arguments
    ///
    /// * `subject` - The profile ID this NFT refers to.
    /// * `issuer` - The authority issuing the NFT (must be trusted).
    /// * `level` - The initial verification level.
    /// * `issued_at` - The timestamp of issuance (e.g., current UNIX time).
    /// * `metadata` - Optional metadata (e.g., document hash).
    /// * `external_url` - Optional URL to external verification source or detail page.
    ///
    /// # Returns
    ///
    /// A new `KycNft` instance with `revoked` set to `false`.
    ///
    /// # Note
    ///
    /// This method does not validate whether the issuer is authorized.
    /// It is recommended to enforce this check externally.
    pub fn new(
        subject: &str,
        issuer: &str,
        level: KycLevel,
        issued_at: u64,
        metadata: Option<String>,
        external_url: Option<String>
    ) -> Self {
        Self {
            subject: subject.to_string(),
            issuer: issuer.to_string(),
            level,
            issued_at,
            revoked: false,
            metadata,
            external_url
        }
    }

    /// Upgrades the verification level of this NFT.
    ///
    /// This method promotes the `level` of the NFT if the new level is higher than the current one.
    /// It also updates the `issued_at` timestamp and resets the `revoked` status to `false`.
    ///
    /// # Arguments
    ///
    /// * `new_level` - The new verification level to assign.
    /// * `new_issued_at` - The updated issuance timestamp (e.g., current UNIX time).
    ///
    /// # Example
    ///
    /// ```rust
    /// use nimble_protocol::core::nft_kyc::model::{KycNft, KycLevel};
    /// use nimble_protocol::core::utils::time::current_time;
    ///
    /// let mut nft = KycNft::new(
    ///     "user-123",
    ///     "atlas-protocol/bank",
    ///     KycLevel::Basic,
    ///     current_time(),
    ///     None,
    ///     None,
    /// );
    ///
    /// nft.upgrade(KycLevel::Advanced, current_time());
    /// assert_eq!(nft.level, KycLevel::Advanced);
    /// assert!(nft.is_active());
    /// ```
    pub fn upgrade(&mut self, new_level: KycLevel, new_issued_at: u64) {
        if new_level > self.level {
            self.level = new_level;
            self.issued_at = new_issued_at;
            self.revoked = false;
        }
    }

    /// Revokes the NFT, marking it as inactive.
    ///
    /// Revoked NFTs are considered invalid for KYC-related purposes
    /// and should no longer be trusted in authorization workflows.
    pub fn revoke(&mut self) {
        self.revoked = true;
    }

    /// Returns `true` if the NFT is still valid (i.e., not revoked).
    pub fn is_active(&self) -> bool {
        !self.revoked
    }
    
    pub fn proof_hash(&self) -> [u8; 32] {
        calculate_proof_hash(self)
    }
}


/// Represents the possible levels of identity verification within the Kycon network.
///
/// Each level reflects the depth of information collected during onboarding or compliance procedures.
/// The levels are ordered, meaning that `Advanced` > `Basic`, and `Institutional` is the highest.
///
/// This enum is typically used in `KycNft` to determine user privileges, onboarding stage,
/// and eligibility for certain actions (e.g., withdrawals, institutional features).
///
/// # Variants
///
/// - `Anonymous`: No verification performed. Default or unverified state.
/// - `Basic`: Basic verification, such as email or phone.
/// - `Advanced`: Full individual verification (e.g., government ID, tax ID, address).
/// - `Institutional`: Corporate or organizational verification with formal compliance.
///
/// # Serialization
///
/// When serialized to JSON (e.g., via API), each variant is converted to lowercase:
/// - `Basic` → `"basic"`
/// - `Institutional` → `"institutional"`
///
/// # Ordering
///
/// The variants implement `Ord`, so you can compare levels:
///
/// ```rust
/// assert!(KycLevel::Advanced > KycLevel::Basic);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KycLevel {
    /// No verification (anonymous or default state).
    Anonymous,

    /// Basic user verification (email and/or phone).
    Basic,

    /// Government-level verification (ID, CPF, proof of address).
    Advanced,

    /// Institutional or corporate-level verification with full compliance.
    Institutional,
}

impl fmt::Display for KycLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            KycLevel::Anonymous => "Anonymous",
            KycLevel::Basic => "Basic",
            KycLevel::Advanced => "Advanced",
            KycLevel::Institutional => "Institutional",
        };
        write!(f, "{}", label)
    }
}
