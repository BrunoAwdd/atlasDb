use atlas_ledger::Ledger;
use atlas_common::transactions::{Transaction, SignedTransaction, signing_bytes};
use atlas_common::env::proposal::Proposal;
use atlas_common::utils::NodeId;
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use std::time::SystemTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup Ledger
    let data_dir = "data/test_split";
    // Clean up previous run
    let _ = std::fs::remove_dir_all(data_dir);
    let ledger = Ledger::new(data_dir).await?;

    // 2. Setup Identities
    let mut csprng = OsRng;
    
    // Payer
    let payer_sk = SigningKey::generate(&mut csprng);
    let payer_pk = payer_sk.verifying_key();
    let payer_addr = atlas_common::address::address::Address::address_from_pk(&payer_pk, "nbex").unwrap();
    let payer_account_key = format!("wallet:{}", payer_addr);

    // Sender (can be same as payer, but let's make them different to be sure)
    let sender_sk = SigningKey::generate(&mut csprng);
    let sender_pk = sender_sk.verifying_key();
    let sender_addr = atlas_common::address::address::Address::address_from_pk(&sender_pk, "nbex").unwrap();

    // Validator (Proposer)
    let validator_sk = SigningKey::generate(&mut csprng);
    let validator_pk = validator_sk.verifying_key();
    let validator_addr = atlas_common::address::address::Address::address_from_pk(&validator_pk, "nbex").unwrap();
    let validator_account_key = format!("wallet:{}", validator_addr);

    println!("Payer: {}", payer_addr);
    println!("Validator: {}", validator_addr);

    // 3. Fund Payer and Sender directly in State
    {
        let mut state = ledger.state.write().await;
        // Fund Payer (Fees)
        let payer_acc = state.accounts.entry(payer_account_key.clone()).or_insert_with(atlas_ledger::core::ledger::account::AccountState::new);
        payer_acc.balances.insert("ATLAS".to_string(), 1_000_000);
        
        // Fund Sender (Transfer Amount)
        // Note: Sender key usually wrapped too if standard
        let sender_account_key = format!("wallet:{}", sender_addr);
        let sender_acc = state.accounts.entry(sender_account_key).or_insert_with(atlas_ledger::core::ledger::account::AccountState::new);
        sender_acc.balances.insert("ATLAS".to_string(), 1_000_000);

        println!("Funded Payer and Sender with 1,000,000 ATLAS");
    }

    // 4. Create Transaction
    let tx = Transaction {
        from: sender_addr.clone(),
        to: "recipient".to_string(), // Doesn't matter for fee test
        amount: 10,
        asset: "ATLAS".to_string(),
        nonce: 1, 
        timestamp: SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        memo: Some("Fee Split Test".to_string()),
    };
    
    // Sign by Sender
    let msg = signing_bytes(&tx);
    let sender_sig = sender_sk.sign(&msg).to_vec();
    let sender_pk_bytes = sender_pk.to_bytes().to_vec();

    // Sign by Payer (Fee Authorization)
    let payer_sig = payer_sk.sign(&msg).to_vec(); // Payer signs transaction hash
    let payer_pk_bytes = payer_pk.to_bytes().to_vec();

    let signed_tx = SignedTransaction {
        transaction: tx,
        signature: sender_sig,
        public_key: sender_pk_bytes,
        fee_payer: Some(payer_addr.clone()),
        fee_payer_signature: Some(payer_sig),
        fee_payer_pk: Some(payer_pk_bytes),
    };

    // 5. Create Proposal
    let content = serde_json::to_string(&vec![signed_tx])?;
    let proposal = Proposal {
        id: "prop-1".to_string(),
        proposer: NodeId(validator_addr.clone()), // Validator is the proposer
        content,
        parent: None,
        height: 1,
        hash: "hash".to_string(),
        prev_hash: "prev".to_string(),
        round: 1,
        time: 0,
        state_root: "".to_string(),
        signature: [0u8; 64],
        public_key: vec![],
    };

    // 6. Execute Transaction
    println!("Executing Transaction...");
    let count = ledger.execute_transaction(&proposal, false).await?;
    println!("Executed {} transactions", count);

    // 7. Verify Balances
    let state = ledger.state.read().await;
    
    // Check Payer Balance
    let payer_bal = state.accounts.get(&payer_account_key).unwrap().balances.get("ATLAS").unwrap();
    println!("Payer Balance: {}", payer_bal);

    // Check Validator Balance
    let validator_bal = state.accounts.get(&validator_account_key).unwrap().balances.get("ATLAS").unwrap();
    println!("Validator Balance: {}", validator_bal);

    // Check System Balance
    let system_bal = state.accounts.get("vault:fees").map(|a| a.balances.get("ATLAS").unwrap_or(&0)).unwrap_or(&0);
    println!("System Balance: {}", system_bal);

    // Assertions
    // Fee Calculation: Base 1000 + Bytes * 10
    // We need to calculate expected fee to verify split
    // Since we don't know exact bytes in this context easily without re-serializing, we check ratios.
    
    assert!(validator_bal > &0, "Validator should have received fees");
    assert!(system_bal > &0, "System should have received fees");
    
    // Check Split Ratio
    // Validator Revenue / System Revenue should be approx 9 (90/10)
    let ratio = *validator_bal as f64 / *system_bal as f64;
    println!("Split Ratio (Validator/System): {}", ratio);
    
    // Allow small rounding error
    if ratio < 8.9 || ratio > 9.1 {
        println!("❌ Split Ratio deviation too high! Expected ~9.0");
        return Err("Incorrect Fee Split".into());
    } else {
        println!("✅ Split Ratio Correct (90/10)");
    }

    Ok(())
}
