use std::{collections::HashMap, sync::Mutex};

use serde::{Deserialize, Serialize};
use atlas_common::{
    address::profile_address::ProfileAddress,
    transactions::TransferRequest, 
    utils::security::generate_seed
};

use crate::{
    identity::identity::generate, 
    profile::profile_type::Profile,
    session::session::Session, 
    vault::vault::VaultData
};

// Declare the submodules
pub mod actions;
pub mod queries;
pub mod auth;

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

// The main Wallet object holding all state and logic
pub struct Wallet {
    session: Option<Session>,
    transfer_map: Mutex<HashMap<String, TransferRequest>>,
}

impl Wallet {
    pub fn new() -> Self {
        Self {
            session: None,
            transfer_map: Mutex::new(HashMap::new()),
        }
    }

    // --- Method calls to action submodule ---
    pub fn load_vault(&mut self, password: String, encoded: Vec<u8>) -> Result<(), String> {
        actions::load_vault(self, password, encoded)
    }

    pub fn sing_transfer(
        &mut self,
        to_address: String,
        amount: u64,
        password: String,
        memo: Option<String>,
    ) -> Result<(String, TransferRequest, Vec<u8>), String> {
        actions::sing_transfer(self, to_address, amount, password, memo)
    }

    pub fn sign_message(&mut self, message: Vec<u8>, password: String) -> Result<String, String> {
        actions::sign_message(self, message, password)
    }

    pub fn switch_profile(&mut self) -> Result<(), String> {
        actions::switch_profile(self)
    }

    // --- Method calls to query submodule ---
    pub fn get_data(&self) -> Result<WalletData, String> {
        queries::get_data(self)
    }

    pub fn selected_account(&self) -> Result<String, String> {
        queries::selected_account(self)
    }

    pub fn validade(&self, transfer: TransferRequest) -> Result<String, String> {
        queries::validade(self, transfer)
    }

    pub fn validate_message(&self, msg: Vec<u8>, signature: &[u8; 64]) -> Result<String, String> {
        queries::validate_message(self, msg, signature)
    }
}

// This function remains as it does not depend on a loaded wallet state.
pub fn create_vault(
    password: String, 
) -> Result<(Vec<u8>, String, String), String> {
    let seed = generate_seed();
    let bundle = generate(&seed, password.clone())
                                    .expect("Failed to generate identity");
    let vault = VaultData::new(1, vec![0u8; 12]);

    let encrypted = vault
                                .save_identity_bundle(&bundle, password)
                                .expect("Failed to save bundle");

    Ok((
        encrypted, 
        bundle.identity.exposed.address().as_str().to_string(), 
        bundle.identity.hidden.address().as_str().to_string()
    ))
}
