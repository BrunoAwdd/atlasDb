use serde::{Deserialize, Serialize};

// Data-only struct for serialization
#[derive(Serialize, Deserialize)]
pub struct AccountData {
    pub address: String,
    pub balance: u64,
}

// Data-only struct for serialization
#[derive(Serialize, Deserialize)]
pub struct WalletData {
    pub exposed: AccountData,
    pub hidden: AccountData,
}
