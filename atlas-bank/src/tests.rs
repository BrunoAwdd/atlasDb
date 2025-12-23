#[cfg(test)]
mod tests {
    use crate::bank::compliance_engine::{service::ComplianceService, kyc::KycRule, traits::ComplianceRule};
    use crate::bank::institution_core::{institution::Institution, registry::InstitutionRegistry};
    use atlas_common::transactions::Transaction;
    use std::sync::Arc;

    #[test]
    fn test_kyc_rule_rejection() {
        let rule = KycRule;
        let tx = Transaction {
            from: "".to_string(), // Invalid
            to: "bob".to_string(),
            amount: 100,
            asset: "USD".to_string(),
            memo: None,
        };

        assert!(rule.check(&tx).is_err());
    }

    #[test]
    fn test_kyc_rule_acceptance() {
        let rule = KycRule;
        let tx = Transaction {
            from: "alice".to_string(),
            to: "bob".to_string(),
            amount: 100,
            asset: "USD".to_string(),
            memo: None,
        };

        assert!(rule.check(&tx).is_ok());
    }

    #[test]
    fn test_compliance_service() {
        let mut service = ComplianceService::new();
        service.add_rule(Arc::new(KycRule));

        let tx_fail = Transaction {
            from: "".to_string(),
            to: "bob".to_string(),
            amount: 100,
            asset: "USD".to_string(),
            memo: None,
        };
        assert!(service.check_compliance(&tx_fail).is_err());

        let tx_ok = Transaction {
            from: "alice".to_string(),
            to: "bob".to_string(),
            amount: 100,
            asset: "USD".to_string(),
            memo: None,
        };
        assert!(service.check_compliance(&tx_ok).is_ok());
    }

    #[test]
    fn test_institution_registry() {
        let mut registry = InstitutionRegistry::new();
        let inst = Institution::new(
            "bank_a".to_string(),
            "Bank A".to_string(),
            "pubkey".to_string(),
            "Tier1".to_string(),
        );

        assert!(registry.add_institution(inst.clone()).is_ok());
        assert!(registry.is_authorized("bank_a"));
        assert!(!registry.is_authorized("bank_b"));

        // Duplicate
        assert!(registry.add_institution(inst).is_err());
    }
}
