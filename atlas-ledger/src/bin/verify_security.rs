use atlas_ledger::interface::api::service::ledger_proto::ledger_service_client::LedgerServiceClient;
use atlas_ledger::interface::api::service::ledger_proto::SubmitTransactionRequest;
use atlas_common::transactions::{Transaction, signing_bytes};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Verify args for port
    let args: Vec<String> = std::env::args().collect();
    let port = args.get(1).unwrap_or(&"50051".to_string()).clone();
    
    let addr = format!("http://0.0.0.0:{}", port);
    println!("Connecting to {}...", addr);

    let mut client = LedgerServiceClient::connect(addr).await?;

    // 1. Setup Identity
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let public_key = signing_key.verifying_key().to_bytes();
    let from_addr = hex::encode(public_key); 

    // 2. Create Transaction
    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let tx = Transaction {
        from: from_addr.clone(),
        to: "recipient_addr".to_string(),
        amount: 100,
        asset: "MEL".to_string(),
        nonce: 1,
        timestamp,
        memo: Some("Security Test".to_string()),
    };

    println!("Testing VALID transaction...");
    // 3. Sign it
    let msg = signing_bytes(&tx);
    let signature = signing_key.sign(&msg);
    let signature_bytes = signature.to_vec();

    // 4. Submit Valid
    let req = SubmitTransactionRequest {
        from: tx.from.clone(),
        to: tx.to.clone(),
        amount: tx.amount.to_string(),
        asset: tx.asset.clone(),
        memo: tx.memo.clone(),
        signature: hex::encode(signature_bytes),
        public_key: hex::encode(public_key),
        nonce: 1,
        timestamp,
        fee_payer: None,
        fee_payer_signature: None,
        fee_payer_public_key: None,
    };

    let resp = client.submit_transaction(req).await;
    match resp {
        Ok(r) => println!("✅ Valid transaction submitted successfully: {:?}", r.into_inner()),
        Err(e) => println!("❌ Valid transaction FAILED: {}", e),
    }

    // 5. Submit INVALID Signature
    println!("\nTesting INVALID transaction...");
    let bad_signature = [0u8; 64]; // Zero signature
    let req_bad = SubmitTransactionRequest {
        from: tx.from.clone(),
        to: tx.to.clone(),
        amount: tx.amount.to_string(),
        asset: tx.asset.clone(),
        memo: tx.memo.clone(),
        signature: hex::encode(bad_signature),
        public_key: hex::encode(public_key),
        nonce: 1,
        timestamp,
        fee_payer: None,
        fee_payer_signature: None,
        fee_payer_public_key: None,
    };

    let resp_bad = client.submit_transaction(req_bad).await;
    match resp_bad {
        Ok(_) => println!("❌ Invalid transaction WAS ACCEPTED (Wait, this is wrong!)"),
        Err(e) => {
            println!("✅ Invalid transaction correctly REJECTED: {}", e.message());
            if e.code() == tonic::Code::Unauthenticated || e.code() == tonic::Code::InvalidArgument {
                println!("   (Error code matches expectation)");
            }
        }
    }

    Ok(())
}
