use crate::core::nft_kyc::model::{KycNft, KycLevel};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializableKycNft {
    pub subject: String,
    pub issuer: String,
    pub level: KycLevel,
    pub issued_at: u64,
    pub revoked: bool,
    pub metadata: Option<String>,
    pub external_url: Option<String>,
}

impl From<&KycNft> for SerializableKycNft {
    fn from(nft: &KycNft) -> Self {
        Self {
            subject: nft.subject.clone(),
            issuer: nft.issuer.clone(),
            level: nft.level,
            issued_at: nft.issued_at,
            revoked: nft.revoked,
            metadata: nft.metadata.clone(),
            external_url: nft.external_url.clone(),
        }
    }
}

impl From<SerializableKycNft> for KycNft {
    fn from(data: SerializableKycNft) -> Self {
        Self {
            subject: data.subject,
            issuer: data.issuer,
            level: data.level,
            issued_at: data.issued_at,
            revoked: data.revoked,
            metadata: data.metadata,
            external_url: data.external_url,
        }
    }
}
/// Serializes a [`KycNft`] into a pretty-printed JSON string.
///
/// # Returns
/// A `Result<String, serde_json::Error>` with the serialized content.
pub fn serialize_nft(nft: &KycNft) -> Result<String, serde_json::Error> {
    let wrapper = SerializableKycNft::from(nft);
    serde_json::to_string_pretty(&wrapper)
}

/// Parses a JSON string and reconstructs a [`KycNft`] object.
///
/// # Returns
/// A `Result<KycNft, serde_json::Error>` if the JSON is valid and complete.

pub fn deserialize_nft(json: &str) -> Result<KycNft, serde_json::Error> {
    let wrapper: SerializableKycNft = serde_json::from_str(json)?;
    Ok(wrapper.into())
}