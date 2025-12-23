use atlas_common::transactions::Transaction;
use atlas_common::error::Result;

/// A rule that must be satisfied for a transaction to be compliant.
pub trait ComplianceRule: Send + Sync {
    /// Returns the name of the rule for logging/debugging.
    fn name(&self) -> &str;

    /// Validates the transaction against this rule.
    /// Returns Ok(()) if compliant, or Err if violation found.
    fn check(&self, tx: &Transaction) -> Result<()>;
}
