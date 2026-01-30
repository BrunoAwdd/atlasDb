use atlas_common::{
    error::{Result, AtlasError},
    transactions::{SignedTransaction, signing_bytes},
    entry::{Leg, LegKind, LedgerEntry},
};
use ed25519_dalek::{VerifyingKey};

pub struct FeeHandler;

impl FeeHandler {
    pub fn apply_fees(
        entry: &mut LedgerEntry,
        st: &SignedTransaction,
        proposer_pk: &[u8],
        proposer_id: &str
    ) -> Result<()> {
        let tx = &st.transaction;
        let fee_payer = st.fee_payer.clone().unwrap_or(tx.from.clone());

        // 1. Calculate Fee
        let base_fee: u64 = 1000;
        let size_bytes = signing_bytes(tx).len() as u64; 
        let byte_fee = size_bytes * 10;
        let total_fee = base_fee + byte_fee;

        // 2. Distribute: 90% Validator, 10% System
        let validator_reward = (total_fee * 90) / 100;
        let system_revenue = total_fee - validator_reward;

        // 3. Legs
        
        // Payer (Debit)
        let payer_account = if fee_payer.starts_with("passivo:wallet:") {
            fee_payer.clone()
        } else {
            format!("passivo:wallet:{}", fee_payer)
        };

        entry.legs.push(Leg {
            account: payer_account,
            asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(), 
            kind: LegKind::Debit, 
            amount: total_fee as u128,
        });

        // Validator Reward (Credit)
        // Derive Validator Wallet from Proposal PK
        let proposer_pk_bytes: [u8; 32] = proposer_pk.try_into()
            .unwrap_or([0u8; 32]);
            
        let proposer_addr = if let Ok(vk) = VerifyingKey::from_bytes(&proposer_pk_bytes) {
             if let Ok(addr) = atlas_common::address::address::Address::address_from_pk(&vk, "nbex") {
                 addr
             } else {
                 proposer_id.to_string()
             }
        } else {
             proposer_id.to_string()
        };

        let validator_account = if proposer_addr.starts_with("passivo:wallet:") {
            proposer_addr
        } else {
            format!("passivo:wallet:{}", proposer_addr)
        };

        entry.legs.push(Leg {
            account: validator_account.clone(),
            asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
            kind: LegKind::Credit,
            amount: validator_reward as u128,
        });

        // System Revenue (Credit)
        entry.legs.push(Leg {
            account: "patrimonio:fees".to_string(),
            asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
            kind: LegKind::Credit,
            amount: system_revenue as u128,
        });
        
        tracing::info!("💸 Fee Distribution: Total={} | Payer={} | Val({})={} | Sys={}", 
            total_fee, fee_payer, validator_account, validator_reward, system_revenue);

        Ok(())
    }
}
