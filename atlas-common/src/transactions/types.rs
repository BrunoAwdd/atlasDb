use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub from: String,
    pub to: String,
    pub amount: u128,
    pub asset: String,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub transaction: Transaction,
    pub signature: Vec<u8>,
    pub public_key: Vec<u8>,
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
