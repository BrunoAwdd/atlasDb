// Declare the submodules
pub mod actions;
pub mod queries;
pub mod auth;
pub mod types;
pub mod factory;
pub mod manager;

pub use types::{AccountData, WalletData};
pub use factory::create_vault;
pub use manager::Wallet;

