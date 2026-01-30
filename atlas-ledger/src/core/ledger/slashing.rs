use atlas_common::{
    error::{Result, AtlasError},
    entry::{Leg, LegKind, LedgerEntry},
};
use crate::Ledger;

impl Ledger {
    /// Puni um validador queimando (confiscando) seus fundos.
    /// Remove o valor do saldo do endereço e creditar em 'patrimonio:slashing' (balanço contábil).
    pub async fn slash_validator(&self, address: &str, amount: u64) -> Result<()> {
        let current_balance = self.get_balance(address, "ATLAS").await?;
        if current_balance == 0 {
            tracing::warn!("⚔️ Slashing falhou: Validador {} já está zerado.", address);
            return Ok(());
        }

        let slash_amt = std::cmp::min(current_balance, amount);
        tracing::info!("⚔️ SLASHING: Punindo {} em {} ATLAS (Saldo: {})", address, slash_amt, current_balance);

        // 1. Debit User Liability (Reduzir passivo = Reduzir grana do user)
        let debit_leg = Leg {
            account: format!("passivo:wallet:{}", address),
            asset: "ATLAS".to_string(),
            kind: LegKind::Debit, // Debit em Liability REDUZ o saldo
            amount: slash_amt as u128,
        };

        // 2. Credit Equity (Slashing Revenue / Burnt)
        let credit_leg = Leg {
            account: "patrimonio:slashing".to_string(),
            asset: "ATLAS".to_string(),
            kind: LegKind::Credit, // Credit em Equity AUMENTA (ganho para a rede/queima)
            amount: slash_amt as u128,
        };

        let mut legs = vec![debit_leg, credit_leg];

        // 3. Shared Slashing Risk: Punish Delegators (10%)
        {
             // Refactoring to hold lock once.
             let mut state = self.state.write().await;
             
             // 3.1 Calculate Delegator Penalty
             let delegated_penalty = state.delegations.slash_delegators(address, 10); // 10% penalty
             if delegated_penalty > 0 {
                 tracing::info!("⚔️ SLASHING SHARED: Punindo delegadores de {} em {} ATLAS (10%)", address, delegated_penalty);
                 // Burn from Staking Pool
                 legs.push(Leg {
                     account: "passivo:wallet:system:staking".to_string(), // Reduce Pool Liability
                     asset: "ATLAS".to_string(),
                     kind: LegKind::Debit, 
                     amount: delegated_penalty as u128,
                 });
                 legs.push(Leg {
                     account: "patrimonio:slashing".to_string(), // Increase Burnt
                     asset: "ATLAS".to_string(),
                     kind: LegKind::Credit,
                     amount: delegated_penalty as u128,
                 });
             }

             let entry_id = format!("slash-{}-{}", address, std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
             
             let entry = LedgerEntry::new(
                 entry_id,
                 legs,
                 "0000000000000000000000000000000000000000000000000000000000000000".to_string(), // No block hash associated yet
                 0,
                 0,
                 Some(format!("SLASHING PENALTY: Disrespectful Behavior")),
             );

             state.apply_entry(entry)
                  .map_err(|e| AtlasError::Other(format!("Failed to apply slashing: {}", e)))?;
        }

        Ok(())
    }
}
