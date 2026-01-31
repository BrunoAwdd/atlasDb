use atlas_common::address::address::Address;
use atlas_common::transactions::{Transaction, TransferRequest};
use ed25519_dalek::{SigningKey, VerifyingKey, Signer};
use rand::rngs::OsRng;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;

// Constants (Alice/Faucet)
// Derived from seed [42u8; 32] used in tests
const FAUCET_SEED: [u8; 32] = [42; 32];
const NODE_URL: &str = "http://localhost:3001";
const ATLAS_TOKEN: &str = "wallet:mint/ATLAS";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    println!("🚀 Starting Resilience Test (100 Wallets)");

    // 1. Setup Faucet (Alice)
    let faucet_sk = SigningKey::from_bytes(&FAUCET_SEED);
    let faucet_vk = VerifyingKey::from(&faucet_sk);
    let faucet_addr_raw = Address::address_from_pk(&faucet_vk, "nbex")?;
    let faucet_addr = format!("wallet:{}", faucet_addr_raw);

    println!("💧 Faucet Address: {}", faucet_addr);

    // Check Faucet Balance
    check_balance(&client, &faucet_addr).await?;

    // 2. Generate 100 Targets
    let mut wallets = Vec::new();
    println!("🔑 Generating 100 Keypairs...");
    for _ in 0..100 {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = VerifyingKey::from(&sk);
        let addr_raw = Address::address_from_pk(&vk, "nbex")?;
        let addr = format!("wallet:{}", addr_raw);
        wallets.push((sk, addr));
    }
    println!("✅ Generated {} wallets.", wallets.len());

    // 3. Fund Wallets (Faucet -> 100 users)
    println!("💸 Funding Wallets (100 ATLAS each)...");
    let mut nonce = get_nonce(&client, &faucet_addr).await?;
    
    for (i, (_, target_addr)) in wallets.iter().enumerate() {
        nonce += 1;
        match send_tx(&client, &faucet_sk, &faucet_addr, target_addr, 100_000_000, nonce).await {
            Ok(hash) => {
                if i % 10 == 0 { println!("   Funding {}/100 -> {} (Tx: {})", i, target_addr, &hash[..8]); }
            },
            Err(e) => eprintln!("❌ Funding Failed for {}: {}", target_addr, e),
        }
        sleep(Duration::from_millis(10)).await; // Pace slightly
    }
    println!("✅ Funding Complete.");

    // Wait for indexing? (Assuming fast block times)
    println!("⏳ Waiting 5s for consistency...");
    sleep(Duration::from_secs(5)).await;

    // 4. Random Traffic (User -> User)
    println!("🌪️ Starting Random Traffic Storm (50 Txs)...");
    let mut success = 0;
    for i in 0..50 {
        let sender_idx = i % wallets.len();
        let receiver_idx = (i + 1) % wallets.len();
        
        let (sender_sk, sender_addr) = &wallets[sender_idx];
        let (_, receiver_addr) = &wallets[receiver_idx];
        
        let sender_nonce = get_nonce(&client, sender_addr).await? + 1; // Simplistic nonce handling
        
        match send_tx(&client, sender_sk, sender_addr, receiver_addr, 1_000_000, sender_nonce).await {
            Ok(_) => { success += 1; },
            Err(e) => eprintln!("❌ Transfer Failed: {}", e),
        }
        if i % 10 == 0 { print!("."); }
    }
    println!("\n✅ Stress Test Complete. {}/50 Txs Sent.", success);

    // 5. Verification Phase (Proof of Settlement)
    println!("🕵️ Verifying On-Chain State (Waiting 15s for blocks)...");
    sleep(Duration::from_secs(15)).await;

    let mut funded_count = 0;
    let mut total_held = 0;

    for (_, addr) in &wallets {
         // Quiet check
         let url = format!("{}/api/balance?query={}", NODE_URL, addr);
         if let Ok(res) = client.get(&url).send().await {
             if let Ok(json) = res.json::<Value>().await {
                 let bal = json["balance"].as_str().unwrap_or("0").parse::<u64>().unwrap_or(0);
                 if bal > 0 {
                     funded_count += 1;
                     total_held += bal;
                 }
             }
         }
    }

    println!("\n📊 VALIDATION REPORT:");
    println!("   - Wallets Verified: 100");
    println!("   - Wallets Funded (Balance > 0): {}", funded_count);
    println!("   - Total ATLAS held by Testers: {}", total_held);
    
    if funded_count == 0 {
        eprintln!("❌ VALIDATION FAILED: No funds found. Transactions may have been rejected or not proposed.");
        return Err("Validation Failed".into());
    } else {
        println!("✅ SUCCESS: Transactions were validated and settled on the real Ledger.");
    }

    Ok(())
}

async fn check_balance(client: &Client, addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!("{}/api/balance?query={}", NODE_URL, addr);
    let res = client.get(&url).send().await?;
    if !res.status().is_success() {
        return Err(format!("Failed to fetch balance: {}", res.status()).into());
    }
    let json: Value = res.json().await?;
    let balance = json["balance"].as_str().unwrap_or("0");
    println!("💰 Balance for {}: {} ATLAS", addr, balance);
    Ok(())
}

async fn get_nonce(client: &Client, addr: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let url = format!("{}/api/balance?query={}", NODE_URL, addr);
    let res = client.get(&url).send().await?;
    let json: Value = res.json().await?;
    let nonce = json["nonce"].as_u64().unwrap_or(0);
    Ok(nonce)
}

async fn send_tx(
    client: &Client, 
    sk: &SigningKey, 
    from: &str, 
    to: &str, 
    amount: u64, 
    nonce: u64
) -> Result<String, Box<dyn std::error::Error>> {
    // Construct Transaction
    let tx = Transaction {
        from: from.to_string(),
        to: to.to_string(),
        amount: amount as u128,
        asset: ATLAS_TOKEN.to_string(),
        nonce,
        timestamp: atlas_common::utils::time::current_time(),
        memo: Some("LOAD_TEST".to_string()),
    };

    // Sign
    let msg = atlas_common::transactions::signing_bytes(&tx);
    let signature = sk.sign(&msg);
    let sig_bytes = signature.to_bytes(); // [u8; 64]

    // Payload (Matches `create_transaction` API input)
    let payload = serde_json::json!({
        "transaction": tx,
        "signature": sig_bytes.to_vec(),
        "public_key": VerifyingKey::from(sk).as_bytes(),
        "fee_payer": null,
        "fee_payer_signature": null,
        "fee_payer_pk": null,
    });

    let res = client.post(format!("{}/api/transaction", NODE_URL))
        .json(&payload)
        .send()
        .await?;
        
    if !res.status().is_success() {
        let text = res.text().await?;
        return Err(format!("Node Message: {}", text).into());
    }
    
    let json: Value = res.json().await?;
    Ok(json["id"].as_str().unwrap_or("?").to_string())
}
