use super::accounts::Account;
use atlas_common::entry::{LedgerEntry, Leg, LegKind}; // Assuming we will move/use these
use serde::{Serialize, Deserialize};

/// Defines the economic nature of a transaction (RFC-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionNature {
    Transfer,
    Issue,
    Burn,
    Fee,
}

pub struct AccountingEngine;

impl AccountingEngine {
    /// Processes a high-level transfer request and generates a double-entry LedgerEntry.
    ///
    /// # Rules (RFC-003)
    /// - **Transfer**: Debit Sender (Liability/Equity) -> Credit Receiver (Liability/Equity)
    ///   (Note: In a bank ledger, user deposits are Liabilities. So sending money reduces the bank's liability to the sender (Debit) and increases it to the receiver (Credit).)
    pub fn process_transfer(
        sender: &str,
        receiver: &str,
        amount: u64,
        asset: &str,
        memo: Option<String>,
    ) -> Result<LedgerEntry, String> {
        // 1. Derive Accounts
        // For a standard transfer between users in a bank ledger:
        // Sender: "passivo:wallet:<sender>" (Liability)
        // Receiver: "passivo:wallet:<receiver>" (Liability)
        
        let sender_account = Account::new(format!("passivo:wallet:{}", sender))?;
        let receiver_account = Account::new(format!("passivo:wallet:{}", receiver))?;

        // 2. Create Legs
        // Debit Sender (Reduce Liability)
        let leg_debit = Leg {
            account: sender_account.as_str().to_string(),
            asset: asset.to_string(),
            kind: LegKind::Debit,
            amount: amount as u128, // LedgerEntry uses u128
        };

        // Credit Receiver (Increase Liability)
        let leg_credit = Leg {
            account: receiver_account.as_str().to_string(),
            asset: asset.to_string(),
            kind: LegKind::Credit,
            amount: amount as u128,
        };

        // 3. Create Entry
        // Entry ID and other metadata should be set by the caller or context
        // For now, we return a partial entry or builder. 
        // Let's assume we return the legs and the caller assembles the entry.
        
        // Actually, let's return a LedgerEntry with a placeholder ID for now.
        let entry = LedgerEntry::new(
            "pending".to_string(), // ID
            vec![leg_debit, leg_credit],
            "hash".to_string(), // Hash
            0, // Height
            0, // Time
            memo,
        );

        Ok(entry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_common::entry::LegKind;

    #[test]
    fn test_process_transfer_creates_balanced_entry() {
        let sender = "alice";
        let receiver = "bob";
        let amount = 100;
        let asset = "USD";

        let entry = AccountingEngine::process_transfer(sender, receiver, amount, asset, None)
            .expect("Should create entry");

        assert_eq!(entry.legs.len(), 2);

        let debit = entry.legs.iter().find(|l| l.kind == LegKind::Debit).unwrap();
        let credit = entry.legs.iter().find(|l| l.kind == LegKind::Credit).unwrap();

        assert_eq!(debit.account, "passivo:wallet:alice");
        assert_eq!(debit.amount, 100);
        assert_eq!(credit.account, "passivo:wallet:bob");
        assert_eq!(credit.amount, 100);
    }
}
