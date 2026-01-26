use atlas_ledger::Ledger;
use atlas_common::transactions::{Transaction, SignedTransaction, signing_bytes};
use atlas_common::env::proposal::Proposal;
use atlas_common::utils::NodeId;
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use std::time::SystemTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = "data/test_treasury";
    let _ = std::fs::remove_dir_all(data_dir);
    let ledger = Ledger::new(data_dir).await?;

    let mut csprng = OsRng;
    
    // 1. Setup Identities
    // Valid Admin Key (from GENESIS_ADMIN_PK definition logic: manually recover hardcoded key?)
    // Actually, `GENESIS_ADMIN_PK` is "03457..." (Hex). We don't have the Private Key for that random string easily unless I generated it.
    // Wait, I hardcoded a random hex string in `genesis.rs` without knowing the Private Key!
    // CRITICAL: To verify "Success", I need the Private Key.
    // I should updated `genesis.rs` with a keypair I KNOW.
    // Let's generate one here and use it. 
    // BUT I can't update `genesis.rs` dynamically in the test.
    
    // STRATEGY: 
    // 1. Generate a new keypair here.
    // 2. We can't overwrite `GENESIS_ADMIN_PK` in the binary.
    // 3. I should have generated a keypair first.
    
    // FIX: I will verify "FAILURE" (Unauthorized) first.
    // Then I will manually update `genesis.rs` with a key I generate now (for development purposes).
    
    // RECREATE ADMIN KEY from known bytes matching updated GENESIS_ADMIN_PK
    // "8a20bab9..." (Hex) is the Public Key. We need the Private Key to sign.
    // Ah, wait. `SigningKey::generate` is random. I cannot regenerate the SAME private key from just the Public Key printed in the previous run.
    // Error in my logic: I printed the PK but lost the SK when the process exited.
    // I need to generate a DETERMINISTIC key pair here, print its PK, update `genesis.rs` AGAIN, and then I can sign properly.
    
    // Quick Fix: Hardcode a Seed for the Admin Key so it's reproducible.
    let seed = [1u8; 32];
    let admin_sk = SigningKey::from_bytes(&seed);
    let admin_pk = admin_sk.verifying_key();
    let admin_pk_hex = hex::encode(admin_pk.to_bytes());
    
    println!("DETERMINISTIC ADMIN PK: {}", admin_pk_hex);
    println!("Please ensure genesis.rs has this PK.");
    
    // Unauthorized Key
    let hacker_sk = SigningKey::generate(&mut csprng);
    let hacker_pk = hacker_sk.verifying_key();
    let hacker_pk_hex = hex::encode(hacker_pk.to_bytes());

    // 2. Fund Treasury (Manually)
    {
        let mut state = ledger.state.write().await;
        // Fund both variants to be sure
        let acc1 = state.accounts.entry("patrimonio:fees".to_string()).or_insert_with(atlas_ledger::core::ledger::account::AccountState::new);
        acc1.balances.insert("ATLAS".to_string(), 1_000_000);
        
        let acc2 = state.accounts.entry("passivo:wallet:patrimonio:fees".to_string()).or_insert_with(atlas_ledger::core::ledger::account::AccountState::new);
        acc2.balances.insert("ATLAS".to_string(), 1_000_000);
    }

    // 3. Create Transaction FROM 'patrimonio:fees'
    let tx = Transaction {
        from: "patrimonio:fees".to_string(),
        to: "recipient".to_string(),
        amount: 100,
        asset: "ATLAS".to_string(),
        nonce: 1,
        timestamp: 0,
        memo: None,
    };
    let msg = signing_bytes(&tx);

    // TEST 1: Hacker Signs
    let hacker_sig = hacker_sk.sign(&msg).to_vec();
    let signed_tx_hacker = SignedTransaction {
        transaction: tx.clone(),
        signature: hacker_sig,
        public_key: hacker_pk.to_bytes().to_vec(),
        fee_payer: None, fee_payer_signature: None, fee_payer_pk: None,
    };
    
    let proposal_hacker = Proposal {
        id: "prop-hacker".to_string(), proposer: NodeId("node1".into()), content: serde_json::to_string(&signed_tx_hacker)?,
        parent: None, height: 1, hash: "h1".into(), prev_hash: "p1".into(), round: 1, time: 0, state_root: "".into(), signature: [0;64], public_key: vec![],
    };

    println!("Attempting Hacker Spend...");
    let res = ledger.execute_transaction(&proposal_hacker, false).await;
    match res {
        Ok(_) => println!("❌ Hacker Spend SUCCEEDED (Should Fail)"),
        Err(e) => println!("✅ Hacker Spend BLOCKED: {}", e),
    }

    // TEST 2: Admin Signs
    // Admin acts as "Sender" (authorizing spend for patrimonio:fees)
    // Note: The public key in the Signature MUST be the Admin Key.
    // The transaction `from` is `patrimonio:fees`.
    let admin_sig = admin_sk.sign(&msg).to_vec();
    let signed_tx_admin = SignedTransaction {
        transaction: tx.clone(),
        signature: admin_sig,
        public_key: admin_pk.to_bytes().to_vec(),
        fee_payer: None, fee_payer_signature: None, fee_payer_pk: None,
    };
    
    let proposal_admin = Proposal {
        id: "prop-admin".to_string(), proposer: NodeId("node1".into()), content: serde_json::to_string(&signed_tx_admin)?,
        parent: None, height: 2, hash: "h2".into(), prev_hash: "h1".into(), round: 2, time: 0, state_root: "".into(), signature: [0;64], public_key: vec![],
    };

    println!("Attempting Admin Spend...");
    let res_admin = ledger.execute_transaction(&proposal_admin, false).await;
    match res_admin {
        Ok(_) => println!("✅ Admin Spend SUCCEEDED (As Expected)"),
        Err(e) => println!("❌ Admin Spend FAILED: {}", e),
    }

    Ok(())
}
