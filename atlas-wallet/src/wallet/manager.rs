use std::{collections::HashMap, sync::Mutex};
use atlas_common::transactions::TransferRequest;

use crate::{
    session::session::Session, 
};

use super::{actions, queries};
use super::types::WalletData;

// The main Wallet object holding all state and logic
pub struct Wallet {
    pub(crate) session: Option<Session>,
    pub(crate) transfer_map: Mutex<HashMap<String, TransferRequest>>,
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
        asset: String,
        memo: Option<String>,
        nonce: u64,
    ) -> Result<(String, TransferRequest, Vec<u8>), String> {
        actions::sing_transfer(self, to_address, amount, asset, memo, nonce)
    }

    pub fn sign_message(&mut self, message: Vec<u8>) -> Result<String, String> {
        actions::sign_message(self, message)
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
