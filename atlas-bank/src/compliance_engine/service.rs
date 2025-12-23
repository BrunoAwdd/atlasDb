use atlas_common::transactions::Transaction;
use atlas_common::error::Result;
use super::traits::ComplianceRule;
use std::sync::Arc;

pub struct ComplianceService {
    rules: Vec<Arc<dyn ComplianceRule>>,
}

impl ComplianceService {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    pub fn add_rule(&mut self, rule: Arc<dyn ComplianceRule>) {
        self.rules.push(rule);
    }

    pub fn check_compliance(&self, tx: &Transaction) -> Result<()> {
        for rule in &self.rules {
            rule.check(tx).map_err(|e| {
                atlas_common::error::AtlasError::Other(format!("Compliance Violation [{}]: {}", rule.name(), e))
            })?;
        }
        Ok(())
    }
}
