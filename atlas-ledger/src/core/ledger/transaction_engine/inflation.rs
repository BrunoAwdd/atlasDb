use atlas_common::{
    error::{Result},
    transactions::{SignedTransaction},
    entry::{Leg, LegKind, LedgerEntry},
};
use ed25519_dalek::VerifyingKey;

pub struct InflationHandler;

impl InflationHandler {
    pub fn apply_inflation(
        entry: &mut LedgerEntry,
        st: &SignedTransaction,
        proposer_pk: &[u8],
        proposer_id: &str
    ) -> Result<()> {
        let tx = &st.transaction;
        let fee_payer = st.fee_payer.clone().unwrap_or(tx.from.clone());

        // 1. Config
        let mint_amount: u128 = 10_000;
        let treasury_mint = 4_000u128;
        let validator_mint = 4_000u128;
        let user_mint = 2_000u128;
        
        // 2. Issuance (Debit Equity/Contra)
        entry.legs.push(Leg {
            account: "patrimonio:issuance".to_string(),
            asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
            kind: LegKind::Debit, // Reduces Equity (New Liability)
            amount: mint_amount,
        });

        // 3. Treasury (Credit)
        entry.legs.push(Leg {
            account: "patrimonio:treasury".to_string(),
            asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
            kind: LegKind::Credit,
            amount: treasury_mint,
        });

        // 4. Validator Bonus (Credit)
        // Recalculating validator address for safety/independence
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
            amount: validator_mint,
        });

        // 5. User Cashback (Credit)
        let user_reward_account = if fee_payer.starts_with("passivo:wallet:") {
            fee_payer.clone()
        } else {
            format!("passivo:wallet:{}", fee_payer)
        };

        entry.legs.push(Leg {
            account: user_reward_account,
            asset: crate::core::ledger::asset::ATLAS_FULL_ID.to_string(),
            kind: LegKind::Credit,
            amount: user_mint,
        });
        
        tracing::info!("🌱 MINT: {} ATLAS distributed (Tre={}, Val={}, User={})", mint_amount, treasury_mint, validator_mint, user_mint);
        
        Ok(())
    }
}
