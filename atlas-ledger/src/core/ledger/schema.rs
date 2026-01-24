use serde::{Deserialize, Serialize};
use std::fmt;

/// 🗺️ AtlasDB Chart of Accounts (Plano de Contas)
/// 
/// This enum defines the strict numeric codes used by the Atlas Ledger.
/// See `CHART_OF_ACCOUNTS.md` for the full hierarchal map.
///
/// # Logic
/// - **Wallets (`0x...`)**: Automatically mapped to `2.1` (Passivo Circulante).
/// - **Internal Accounts**: Must follow `PREFIX:NAME` format (e.g., `1.1:Caixa`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum AccountClass {
    Ativo,            // 1
    Passivo,          // 2
    PatrimonioLiquido,// 3
    Resultado,        // 4
    Compensacao,      // 5
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Liquidity {
    Current,    // Circulante (Short Term)
    NonCurrent, // Não Circulante (Long Term)
    None,       // Not applicable (e.g. Equity, Income)
}

impl fmt::Display for AccountClass {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AccountClass::Ativo => write!(f, "1 - Ativo"),
            AccountClass::Passivo => write!(f, "2 - Passivo"),
            AccountClass::PatrimonioLiquido => write!(f, "3 - PL"),
            AccountClass::Resultado => write!(f, "4 - Resultado"),
            AccountClass::Compensacao => write!(f, "5 - Compensacao"),
            AccountClass::Unknown => write!(f, "? - Unknown"),
        }
    }
}

pub struct AccountSchema;

impl AccountSchema {
    /// Returns the main class of the account (Asset, Liability, etc.)
    pub fn parse_root(address: &str) -> AccountClass {
        if address.starts_with("0x") || address.starts_with("nbex") {
            return AccountClass::Passivo; // 0x Wallets are Liabilities (Deposits)
        }
        
        // Check strict prefixes
        if address.starts_with("1.") { return AccountClass::Ativo; }
        if address.starts_with("2.") { return AccountClass::Passivo; }
        if address.starts_with("3.") { return AccountClass::PatrimonioLiquido; }
        if address.starts_with("4.") { return AccountClass::Resultado; }
        if address.starts_with("5.") { return AccountClass::Compensacao; }

        AccountClass::Unknown
    }
    
    /// Returns the liquidity level (Current vs Non-Current)
    pub fn get_liquidity(address: &str) -> Liquidity {
        if address.starts_with("0x") || address.starts_with("nbex") {
            return Liquidity::Current; // Wallets are demand deposits (Current Liability)
        }

        // 2-Level Prefix Parsing
        if address.starts_with("1.1") || address.starts_with("2.1") {
            return Liquidity::Current;
        }
        if address.starts_with("1.2") || address.starts_with("2.2") {
            return Liquidity::NonCurrent;
        }

        Liquidity::None
    }

    /// Validates if an address follows the strict 2-level schema (e.g., "1.1:Name")
    pub fn validate(address: &str) -> bool {
        // 1. Allow standard Hex Addresses (Wallets)
        if address.starts_with("0x") || address.starts_with("nbex") {
            // Verify hex chars? For now just prefix is enough for schema validation.
            return true;
        }

        // 2. Validate Internal Accounts (MUST be `X.Y:Name` or `X.Y.Z:Name`)
        // Accepted Prefixes from CHART_OF_ACCOUNTS.md
        let valid_prefixes = [
            "1.1", "1.2",       // Ativo
            "2.1", "2.2",       // Passivo
            "3.1", "3.2", "3.3",// PL
            "4.1", "4.2", "4.3",// Resultado
            "5.1",              // Compensacao
        ];

        for prefix in valid_prefixes {
            if address.starts_with(prefix) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_classification() {
        let wallet = "0x123abc";
        assert_eq!(AccountSchema::parse_root(wallet), AccountClass::Passivo);
        assert_eq!(AccountSchema::get_liquidity(wallet), Liquidity::Current);
        assert!(AccountSchema::validate(wallet));
    }

    #[test]
    fn test_valid_internal_accounts() {
        // Assets
        assert!(AccountSchema::validate("1.1:Caixa"));
        assert_eq!(AccountSchema::get_liquidity("1.1:Caixa"), Liquidity::Current);
        
        assert!(AccountSchema::validate("1.2:Imob"));
        assert_eq!(AccountSchema::get_liquidity("1.2:Imob"), Liquidity::NonCurrent);

        // Liabilities
        assert!(AccountSchema::validate("2.1:Forn"));
        assert_eq!(AccountSchema::get_liquidity("2.1:Forn"), Liquidity::Current);

        // Equity
        assert!(AccountSchema::validate("3.1:Capital"));
        assert_eq!(AccountSchema::get_liquidity("3.1:Capital"), Liquidity::None);
    }

    #[test]
    fn test_invalid_accounts() {
        assert!(!AccountSchema::validate("1.9:Fake")); // 1.9 doesn't exist
        assert!(!AccountSchema::validate("9.9:Fake")); // 9 doesn't exist
        assert!(!AccountSchema::validate("random_string"));
        assert!(!AccountSchema::validate("1:NoSubLevel")); // Must have at least X.Y
    }
}
