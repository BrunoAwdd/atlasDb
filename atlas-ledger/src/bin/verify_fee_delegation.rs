use atlas_ledger::interface::api::service::ledger_proto::ledger_service_client::LedgerServiceClient;
use atlas_ledger::interface::api::service::ledger_proto::SubmitTransactionRequest;
use atlas_common::transactions::{Transaction, signing_bytes};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;
use std::time::SystemTime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let port = args.get(1).unwrap_or(&"50051".to_string()).clone();
    let addr = format!("http://0.0.0.0:{}", port);
    println!("Connecting to {}...", addr);

    let mut client = LedgerServiceClient::connect(addr).await?;

    // 1. Setup Sender and Fee Payer Identities
    let mut csprng = OsRng;
    
    // Sender
    let sender_sk = SigningKey::generate(&mut csprng);
    let sender_pk = sender_sk.verifying_key();
    let sender_addr = atlas_common::address::address::Address::address_from_pk(&sender_pk, "nbex").unwrap();
    
    // Fee Payer (Different from Sender)
    let payer_sk = SigningKey::generate(&mut csprng);
    let payer_pk = payer_sk.verifying_key();
    let payer_addr = atlas_common::address::address::Address::address_from_pk(&payer_pk, "nbex").unwrap();
    
    // Recipient
    let recipient_sk = SigningKey::generate(&mut csprng);
    let recipient_pk = recipient_sk.verifying_key();
    let recipient_addr = atlas_common::address::address::Address::address_from_pk(&recipient_pk, "nbex").unwrap();

    println!("Sender: {}", sender_addr);
    println!("Payer: {}", payer_addr);
    println!("Recipient: {}", recipient_addr);

    // 2. Create Transaction (Sender sings it)
    let timestamp = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let tx = Transaction {
        from: sender_addr.clone(),
        to: recipient_addr.clone(),
        amount: 50,
        asset: "ATLAS".to_string(),
        nonce: 1, // Assuming nonce 1 is valid (fresh account)
        timestamp,
        memo: Some("Delegated Fee Test".to_string()),
    };
    
    let msg = signing_bytes(&tx);
    let sender_sig = sender_sk.sign(&msg);

    // 3. Fee Payer Signs the SAME transaction to authorize payment
    let payer_sig = payer_sk.sign(&msg);
    
    // 4. Submit with Delegation
    let req = SubmitTransactionRequest {
        from: tx.from.clone(),
        to: tx.to.clone(),
        amount: tx.amount.to_string(),
        asset: tx.asset.clone(),
        memo: tx.memo.clone(),
        signature: hex::encode(sender_sig.to_bytes()), 
        public_key: hex::encode(sender_pk.to_bytes()),
        nonce: 1,
        timestamp,
        // Fee Delegation Fields
        fee_payer: Some(payer_addr.clone()),
        fee_payer_signature: Some(hex::encode(payer_sig.to_bytes())),
        fee_payer_public_key: Some(hex::encode(payer_pk.to_bytes())),
    };

    println!("Submitting Delegated Transaction...");
    let resp = client.submit_transaction(req).await;
    
    match resp {
        Ok(r) => println!("✅ Transaction Submitted Successfully: {:?}", r.into_inner()),
        Err(e) => println!("❌ Transaction Failed: {}", e),
    }

    Ok(())
}
