use serde::{Deserialize, Serialize};

pub const SYSTEM_MINT_ISSUER: &str = "wallet:mint";
pub const ATLAS_SYMBOL: &str = "ATLAS";
// Strict ID for the native token
pub const ATLAS_FULL_ID: &str = "wallet:mint/ATLAS";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDefinition {
    /// Who issued this asset? (The "Mint" authority)
    /// MUST correspond to an Institution ID in `atlas-bank`.
    pub issuer: String,
    
    // asset_type removed. Ledger is agnostic.
    
    /// Human readable name ("United States Dollar", "Petrobras Stock")
    pub name: String,
    
    /// Ticker Symbol ("USD", "PETR4")
    pub symbol: String,

    /// Standard Classification (e.g., "ISO4217:USD", "ERC20:0x...", "Commodity:GOLD")
    /// Allows UI/Wallets to group distinct assets (Circle:USD, Tether:USD) under a common view.
    pub asset_standard: Option<String>,
    
    /// Precision
    pub decimals: u8,
    
    /// Metadata hash or URL
    pub resource_url: Option<String>,
}

impl AssetDefinition {
    /// Returns the unique namespaced identifier: `issuer_id/symbol`
    pub fn id(&self) -> String {
        format!("{}/{}", self.issuer, self.symbol)
    }

    pub fn new(
        issuer: String,
        name: String,
        symbol: String,
        decimals: u8,
        asset_standard: Option<String>,
    ) -> Self {
        Self {
            issuer,
            name,
            symbol,
            decimals,
            asset_standard,
            resource_url: None,
        }
    }

    /// strict validation of parameters
    pub fn validate(&self) -> Result<(), String> {
        if self.issuer.trim().is_empty() { return Err("Issuer cannot be empty".to_string()); }
        if self.symbol.trim().is_empty() { return Err("Symbol cannot be empty".to_string()); }
        if self.name.trim().is_empty() { return Err("Name cannot be empty".to_string()); }
        
        // Basic symbol validation (alphanumeric check could be added)
        if self.symbol.len() > 10 {
            return Err("Symbol is too long".to_string());
        }

        Ok(())
    }
}
