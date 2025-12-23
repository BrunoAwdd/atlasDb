use atlas_common::transactions::Transaction;
use atlas_common::error::{Result, AtlasError};
use super::traits::ComplianceRule;

pub struct KycRule;

impl ComplianceRule for KycRule {
    fn name(&self) -> &str {
        "KYC/AML Rule"
    }

    fn check(&self, tx: &Transaction) -> Result<()> {
        // Simple check: Sender and Receiver addresses must not be empty.
        // In a real implementation, this would check against a KYC database or state.
        
        if tx.from.is_empty() {
             return Err(AtlasError::Other("Sender address cannot be empty".to_string()));
        }
        
        if tx.to.is_empty() {
             return Err(AtlasError::Other("Receiver address cannot be empty".to_string()));
        }

        // Placeholder for more complex checks (e.g., checking if 'from' is frozen)
        // For now, valid.
        
        Ok(())
    }
}
