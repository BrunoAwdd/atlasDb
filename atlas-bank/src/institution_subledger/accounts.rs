use serde::{Serialize, Deserialize};
use std::fmt;

/// Represents the five main classes of accounts in the double-entry system (RFC-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    Asset,
    Liability,
    Equity,
    Revenue,
    Expense,
}

impl fmt::Display for AccountType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountType::Asset => write!(f, "ativo"),
            AccountType::Liability => write!(f, "passivo"),
            AccountType::Equity => write!(f, "patrimonio"),
            AccountType::Revenue => write!(f, "receita"),
            AccountType::Expense => write!(f, "despesa"),
        }
    }
}

impl AccountType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ativo" | "asset" => Some(AccountType::Asset),
            "passivo" | "liability" => Some(AccountType::Liability),
            "patrimonio" | "equity" => Some(AccountType::Equity),
            "receita" | "revenue" => Some(AccountType::Revenue),
            "despesa" | "expense" => Some(AccountType::Expense),
            _ => None,
        }
    }
}

/// Represents a canonical account in the Chart of Accounts.
/// Format: `type:subtype:detail` (e.g., `ativo:wallet:alice`)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Account(String);

impl Account {
    pub fn new(path: String) -> Result<Self, String> {
        if !Self::is_valid(&path) {
            return Err(format!("Invalid account path: {}", path));
        }
        Ok(Self(path))
    }

    pub fn is_valid(path: &str) -> bool {
        // Allow numeric codes (1.1:Name)
        if path.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            return true;
        }
        // Allow Blockchain Hashes (0x...)
        if path.starts_with("0x") {
            return true;
        }

        let parts: Vec<&str> = path.split(':').collect();
        if parts.len() < 2 {
            return false;
        }
        AccountType::from_str(parts[0]).is_some()
    }

    pub fn account_type(&self) -> AccountType {
        // Handle numeric codes
        if let Some(first_char) = self.0.chars().next() {
            if first_char.is_ascii_digit() {
                return match first_char {
                    '1' => AccountType::Asset,
                    '2' => AccountType::Liability,
                    '3' => AccountType::Equity,
                    '4' => AccountType::Revenue, // Mapping Result to Revenue/Income generic
                    '5' => AccountType::Expense, // Mapping Compensation/Expense generic
                    _ => AccountType::Liability, // Default fallback
                };
            }
        }
        
        // Handle Blockchain Hashes -> Liability (Customer Funds)
        if self.0.starts_with("0x") {
            return AccountType::Liability;
        }

        let parts: Vec<&str> = self.0.split(':').collect();
        AccountType::from_str(parts[0]).unwrap() // Safe because of new() check
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
