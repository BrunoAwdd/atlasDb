use serde::{Deserialize, Serialize};

/// Represents a portable Verifiable Credential for KYC/Identity.
/// This structure is designed to be serialized (JSON/JWT) and stored off-chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Credential {
    /// Unique ID of the credential (e.g., UUID or DID)
    pub id: String,
    
    /// The subject ID (e.g., User's Public Key / DID)
    pub subject: String,
    
    /// The issuer ID (e.g., Bank's Public Key / DID)
    pub issuer: String,
    
    /// The verification level (e.g., "Basic", "Advanced", "Institutional")
    pub level: String,
    
    /// Issuance timestamp (UNIX epoch)
    pub issued_at: u64,
    
    /// Expiration timestamp (optional)
    pub expires_at: Option<u64>,
    
    /// Cryptographic signature of the issuer (proving integrity)
    pub signature: Option<String>,
}

impl Credential {
    pub fn new(id: String, subject: String, issuer: String, level: String, issued_at: u64) -> Self {
        Self {
            id,
            subject,
            issuer,
            level,
            issued_at,
            expires_at: None,
            signature: None,
        }
    }
    
    // Placeholder for signing/verification logic (would use crate::crypto usually)
    pub fn sign(&mut self, _signer_key: &str) {
        // Implementation would create a signature over the fields
        self.signature = Some("dummy_signature".to_string());
    }
    
    pub fn is_valid_signature(&self, _issuer_key: &str) -> bool {
        // Check signature
        true 
    }
}
