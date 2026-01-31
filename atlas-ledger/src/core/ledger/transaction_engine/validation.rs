use atlas_common::{
    error::{Result, AtlasError},
    transactions::{Transaction, SignedTransaction, signing_bytes},
    genesis::GENESIS_ADMIN_PK
};
use ed25519_dalek::{Verifier, VerifyingKey, Signature};
use crate::Ledger;

pub struct ValidationHandler;

impl ValidationHandler {
    /// Validates signatures for Sender and Fee Payer.
    pub fn validate_signatures(st: &SignedTransaction) -> Result<()> {
        let tx = &st.transaction;

        // 1. Sender Signature
        if !st.signature.is_empty() && !st.public_key.is_empty() {
             let verifying_key = VerifyingKey::from_bytes(st.public_key.as_slice().try_into().unwrap_or(&[0u8; 32]))
                .map_err(|e| AtlasError::Other(format!("Invalid public key: {}", e)))?;
             let signature = Signature::from_slice(&st.signature)
                .map_err(|e| AtlasError::Other(format!("Invalid signature format: {}", e)))?;
             let msg = signing_bytes(tx);
             
             // Verify Sender Signature
             if verifying_key.verify(&msg, &signature).is_err() {
                 tracing::error!("❌ Signature Verification Failed for tx from {}", tx.from);
                 return Err(AtlasError::Other("Invalid transaction signature".to_string()));
             }
             
             // Verify Sender Address matches Public Key
             if let Ok(address) = atlas_common::address::address::Address::address_from_pk(&verifying_key, "nbex") {
                 if address != tx.from {
                     tracing::warn!("⚠️ Sender Address Mismatch: Claimed {} vs Derived {}", tx.from, address);
                 }
             }

             // 1.1 SYSTEM ACCOUNT PROTECTION
             if tx.from.starts_with("vault:") {
                 let provided_pk_hex = hex::encode(st.public_key.clone());
                 if provided_pk_hex != GENESIS_ADMIN_PK {
                     tracing::error!("❌ Unauthorized Treasury Spend! PK {} != Admin {}", provided_pk_hex, GENESIS_ADMIN_PK);
                     return Err(AtlasError::Other("Unauthorized: System accounts require Admin Key".to_string()));
                 }
                 tracing::info!("🛡️  Admin Action: Spending from {}", tx.from);
             }
        } else {
             println!("⚠️ Executing unsigned transaction (legacy path)");
        }

        // 2. Fee Payer Signature
        let fee_payer = st.fee_payer.clone().unwrap_or(tx.from.clone());
        
        if let (Some(payer_sig_bytes), Some(payer_pk_bytes)) = (st.fee_payer_signature.as_ref(), st.fee_payer_pk.as_ref()) {
            let payer_vk = VerifyingKey::from_bytes(payer_pk_bytes.as_slice().try_into().unwrap_or(&[0u8; 32]))
                .map_err(|e| AtlasError::Other(format!("Invalid fee payer public key: {}", e)))?;
            let payer_sig = Signature::from_slice(payer_sig_bytes)
                .map_err(|e| AtlasError::Other(format!("Invalid fee payer signature format: {}", e)))?;
            
            let msg = signing_bytes(tx);
            
            if payer_vk.verify(&msg, &payer_sig).is_err() {
                tracing::error!("❌ Fee Payer Signature Verification Failed for payer {}", fee_payer);
                return Err(AtlasError::Other("Invalid fee payer signature".to_string()));
            }
            
            if let Ok(address) = atlas_common::address::address::Address::address_from_pk(&payer_vk, "nbex") {
                if address != fee_payer {
                     tracing::error!("❌ Fee Payer Address Mismatch: Claimed {} vs Derived {}", fee_payer, address);
                     return Err(AtlasError::Other("Fee payer address mismatch".to_string()));
                }
            }
        } else if st.fee_payer.is_some() {
             tracing::error!("❌ Fee Payer {} claimed but no signature provided!", fee_payer);
             return Err(AtlasError::Other("Missing fee payer signature".to_string()));
        }

        Ok(())
    }

    /// Validates asset existence in Ledger.
    pub async fn validate_asset(ledger: &Ledger, asset_id: &str) -> Result<()> {
        if asset_id != crate::core::ledger::asset::ATLAS_FULL_ID {
            let state_guard = ledger.state.read().await;
            if !state_guard.assets.contains_key(asset_id) {
                 tracing::error!("❌ Unknown Asset: {}", asset_id);
                 return Err(AtlasError::Other(format!("Asset '{}' is not registered.", asset_id)));
            }
        }
        Ok(())
    }

    /// Validates Nonce (Stateful).
    pub async fn validate_nonce(ledger: &Ledger, tx: &Transaction) -> Result<u64> {
        let state = ledger.state.read().await;
        
        let account_nonce = if let Some(acc) = state.accounts.get(&tx.from) {
            acc.nonce
        } else if let Some(acc) = state.accounts.get(&format!("wallet:{}", tx.from)) {
            acc.nonce
        } else {
            0
        };

        tracing::info!("🔢 Nonce Check: Account={} | Stored={} | Tx={} | Expected={}", 
            tx.from, account_nonce, tx.nonce, account_nonce + 1);

        if tx.nonce != account_nonce + 1 {
            tracing::error!("❌ Nonce Mismatch! Account={} Expected={} Got={}", tx.from, account_nonce + 1, tx.nonce);
            return Err(AtlasError::Other(format!(
                "Invalid Nonce: Expected {}, got {}. (Account: {})", 
                account_nonce + 1, tx.nonce, tx.from
            )));
        }

        Ok(account_nonce)
    }
}
