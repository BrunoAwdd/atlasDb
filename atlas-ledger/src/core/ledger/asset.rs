use serde::{Deserialize, Serialize};

pub const SYSTEM_MINT_ISSUER: &str = "passivo:wallet:mint";
pub const ATLAS_SYMBOL: &str = "ATLAS";
// Strict ID for the native token
pub const ATLAS_FULL_ID: &str = "passivo:wallet:mint/ATLAS";

/// Defines the category of the asset for accounting purposes.
/// STRICTLY linked to `CHART_OF_ACCOUNTS.md`.
/// Uses universal numeric codes (A=Asset, L=Liability, EQ=Equity, OFF=OffBalance)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssetType {
    // 1. ATIVO (Assets)
    // 1.1 Circulante
    A1_1_1, // Caixa e Equivalentes
    A1_1_2, // Contas a Receber
    // 1.2 Não Circulante
    A1_2_1, // Realizável a Longo Prazo
    A1_2_2, // Investimentos
    A1_2_3, // Imobilizado
    A1_2_4, // Intangível

    // 2. PASSIVO (Liabilities)
    // 2.1 Circulante
    L2_1_1, // Fornecedores / Contas a Pagar
    L2_1_2, // Obrigações Fiscais
    L2_1_3, // Obrigações com Clientes (Custódia/Deposits)
    // 2.2 Não Circulante
    L2_2_1, // Empréstimos LP

    // 3. PATRIMÔNIO LÍQUIDO (Equity)
    EQ3_1, // Capital Social
    EQ3_2, // Reservas
    EQ3_3, // Ajustes

    // 4. RESULTADO
    R4_1, // Receitas Operacionais
    R4_2, // Custos
    R4_3, // Despesas Operacionais

    // 5. OFF-BALANCE
    OFF5_1, // Custody Assets
    OFF5_2, // Guarantees
}

impl AssetType {
    /// Returns the high-level prefix (e.g., "1.1")
    pub fn get_prefix(&self) -> &'static str {
        match self {
            AssetType::A1_1_1 | AssetType::A1_1_2 => "1.1",
            AssetType::A1_2_1 | AssetType::A1_2_2 | AssetType::A1_2_3 | AssetType::A1_2_4 => "1.2",
            AssetType::L2_1_1 | AssetType::L2_1_2 | AssetType::L2_1_3 => "2.1",
            AssetType::L2_2_1 => "2.2",
            AssetType::EQ3_1 => "3.1",
            AssetType::EQ3_2 => "3.2",
            AssetType::EQ3_3 => "3.3",
            AssetType::R4_1 => "4.1",
            AssetType::R4_2 => "4.2",
            AssetType::R4_3 => "4.3",
            AssetType::OFF5_1 => "5.1",
            AssetType::OFF5_2 => "5.2",
        }
    }

    /// Returns the default specific COA code for this asset type.
    /// Used when auto-assigning sub-accounts.
    pub fn default_coa_code(&self) -> &'static str {
        match self {
            AssetType::A1_1_1 => "1.1.1",    // Caixa e Equivalentes
            AssetType::A1_1_2 => "1.1.2",    // Contas a Receber
            AssetType::A1_2_1 => "1.2.1",    // Realizável a Longo Prazo
            AssetType::A1_2_2 => "1.2.2",    // Investimentos (Assumption)
            AssetType::A1_2_3 => "1.2.3",    // Imobilizado
            AssetType::A1_2_4 => "1.2.4",    // Intangível
            AssetType::L2_1_1 => "2.1.1",    // Contas a Pagar
            AssetType::L2_1_2 => "2.1.2",    // Obrigações Fiscais
            AssetType::L2_1_3 => "2.1.3",    // Obrigações com Clientes (Custódia/Deposits)
            AssetType::L2_2_1 => "2.2.1",    // Empréstimos LP
            AssetType::EQ3_1 => "3.1",       // Capital Social
            AssetType::EQ3_2 => "3.2",       // Reservas
            AssetType::EQ3_3 => "3.3",       // Ajustes
            AssetType::R4_1 => "4.1",        // Receitas
            AssetType::R4_2 => "4.2",        // Custos
            AssetType::R4_3 => "4.3",        // Despesas
            AssetType::OFF5_1 => "5.1",      // Ativos de Terceiros
            AssetType::OFF5_2 => "5.2",      // Garantias
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDefinition {
    /// Who issued this asset? (The "Mint" authority)
    /// MUST correspond to an Institution ID in `atlas-bank`.
    pub issuer: String,
    
    /// Type of asset linked to Chart of Accounts logic
    pub asset_type: AssetType,
    
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
        asset_type: AssetType,
        name: String,
        symbol: String,
        decimals: u8,
        asset_standard: Option<String>,
    ) -> Self {
        Self {
            issuer,
            asset_type,
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
