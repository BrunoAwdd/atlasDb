use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: u128,
    pub asset: String,
    pub nonce: u64,
    pub timestamp: u64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    #[serde(with = "hex::serde")]
    pub signature: Vec<u8>,
    #[serde(with = "hex::serde")]
    pub public_key: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fee_payer: Option<String>,
    #[serde(with = "self::hex_opt", default, skip_serializing_if = "Option::is_none")]
    pub fee_payer_signature: Option<Vec<u8>>,
    #[serde(with = "self::hex_opt", default, skip_serializing_if = "Option::is_none")]
    pub fee_payer_pk: Option<Vec<u8>>,
}

mod hex_opt {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(data: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match data {
            Some(v) => hex::serde::serialize(v, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // We visit as Option<String> because hex::serde expects a string to behave like FromHex
        let s: Option<String> = Option::deserialize(deserializer)?;
        match s {
            Some(s) => hex::decode(s).map(Some).map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

pub fn signing_bytes(tx: &Transaction) -> Vec<u8> {
    bincode::serialize(tx).unwrap()
}

impl SignedTransaction {
    /// Performs stateless validation checks using TransactionValidator.
    pub fn validate_stateless(&self) -> Result<(), String> {
        crate::transactions::validation::TransactionValidator::validate_stateless(self)
    }
}
