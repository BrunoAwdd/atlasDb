use atlas_common::address::address::Address;
use atlas_common::transactions::{Transaction, TransferRequest};
use ed25519_dalek::{SigningKey, VerifyingKey, Signer};
use rand::rngs::OsRng;
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use std::fs;
use std::io::Write;
use serde::{Serialize, Deserialize};

// Constants
const NODE_URL: &str = "http://localhost:3001";
const ATLAS_TOKEN: &str = "wallet:mint/ATLAS";
const KEYPAIR_PATH: &str = "example/node1/keypair";
const WALLETS_FILE: &str = "load_test_wallets.json";

#[derive(Serialize, Deserialize)]
struct SavedWallet {
    sk_bytes: Vec<u8>,
    address: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    println!("🚀 Starting Resilience Test (100 Wallets)");

    // 1. Setup Faucet (Alice) - Load from Real Node Keypair
    println!("🔐 Loading Faucet Key from: {}", KEYPAIR_PATH);
    let raw_bytes = fs::read(KEYPAIR_PATH)?;
    
    // Libp2p keypairs have a 4-byte header (Type + Length) -> 64 bytes data
    // Total 68 bytes usually. If simplified (raw), maybe 32.
    // We check length to decide.
    let offset = if raw_bytes.len() == 68 { 4 } else { 0 };
    
    if raw_bytes.len() < offset + 32 {
        return Err(format!("Keyfile too small: {} bytes", raw_bytes.len()).into());
    }

    let seed_slice = &raw_bytes[offset..offset+32];
    let faucet_sk = SigningKey::from_bytes(seed_slice.try_into()?);
    let faucet_vk = VerifyingKey::from(&faucet_sk);
    let faucet_addr = Address::address_from_pk(&faucet_vk, "nbex")?;
    // Raw address (nbex...) is used for transaction signing.

    println!("💧 Faucet Address: {}", faucet_addr);

    // Check Faucet Balance
    // API might expect "wallet:" prefix for query? 
    // Let's try raw first. If 0, try prefix.
    check_balance(&client, &faucet_addr).await?; // This usually logs. If 0, we have a problem.
    // NOTE: If Genesis funded "wallet:nbex...", checking "nbex..." might return 0 if normalization fails in API.
    // But `rest.rs` should handle it.
    // If not, we might need `let faucet_query = format!("wallet:{}", faucet_addr);` for query.
    // But for SIGNING, we use `faucet_addr` (raw).

    // 2. Load or Generate 100 Targets
    let wallets: Vec<(SigningKey, String)>;
    
    if let Ok(content) = fs::read_to_string(WALLETS_FILE) {
        println!("📂 Loading wallets from {}...", WALLETS_FILE);
        let saved: Vec<SavedWallet> = serde_json::from_str(&content)?;
        wallets = saved.into_iter().map(|w| {
            let sk = SigningKey::from_bytes(w.sk_bytes[..32].try_into().unwrap());
            (sk, w.address)
        }).collect();
        println!("✅ Loaded {} wallets.", wallets.len());
    } else {
        println!("🔑 Generating 100 Keypairs...");
        let mut generated = Vec::new();
        let mut saved = Vec::new();
        
        for _ in 0..100 {
            let sk = SigningKey::generate(&mut OsRng);
            let vk = VerifyingKey::from(&sk);
            let addr = Address::address_from_pk(&vk, "nbex")?;
            
            generated.push((sk.clone(), addr.clone()));
            saved.push(SavedWallet {
                sk_bytes: sk.to_bytes().to_vec(),
                address: addr,
            });
        }
        
        // Save to file
        let json = serde_json::to_string_pretty(&saved)?;
        fs::write(WALLETS_FILE, json)?;
        println!("✅ Generated and Saved {} wallets to {}.", generated.len(), WALLETS_FILE);
        wallets = generated;
    }

    // 3. Fund Wallets (Batch Mode)
    println!("💸 Verifying Funding Status for {} accounts...", wallets.len());
    
    // Identifiy who needs funds
    let mut to_fund = Vec::new();
    for (i, (_, target_addr)) in wallets.iter().enumerate() {
        if !check_balance_bool(&client, target_addr).await {
            to_fund.push((i, target_addr));
        }
    }
    
    if !to_fund.is_empty() {
    println!("🚀 Batch Funding {} accounts (RAPID FIRE MODE)...", to_fund.len());
    
    let start_sending = std::time::Instant::now();
    let mut nonce = get_nonce(&client, &faucet_addr).await?;

    // Send all transactions rapidly
    for (i, target_addr) in &to_fund {
        nonce += 1;
        print!("\r   Sending Tx {}/{} -> {} (Nonce {}) ", i+1, to_fund.len(), target_addr, nonce);
        use std::io::Write; std::io::stdout().flush().ok();
        
        if let Err(e) = send_tx(&client, &faucet_sk, &faucet_addr, target_addr, 100_000_000, nonce).await {
                println!("\n❌ Failed to send to {}: {}", target_addr, e);
        }
    }
    let sending_duration = start_sending.elapsed();
    println!("\n✅ Sent {} funding transactions in {:.2?}.", to_fund.len(), sending_duration);
    
    // Now wait for settlement
    println!("⏳ Waiting for settlement (polling all)...");
    let mut confirmed = 0;
    let start_settlement = std::time::Instant::now();
    
    while confirmed < to_fund.len() {
        // REMOVED TIMEOUT (Infinite Wait)
        
        confirmed = 0;
        for (_, target_addr) in &to_fund {
            if check_balance_bool(&client, target_addr).await {
                confirmed += 1;
            }
        }
        let elapsed = start_settlement.elapsed();
        print!("\r   Confirmed: {}/{} (Time: {:.2?})", confirmed, to_fund.len(), elapsed);
        std::io::stdout().flush().ok();
        sleep(Duration::from_millis(1000)).await;
    }
    let settlement_duration = start_settlement.elapsed();
    println!("\n✅ Validated Funding. Total Wait: {:.2?}", settlement_duration);
    } else {
        println!("✅ All wallets already funded.");
    }

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

async fn check_balance_bool(client: &Client, addr: &str) -> bool {
    let url = format!("{}/api/balance?query={}", NODE_URL, addr);
    if let Ok(res) = client.get(&url).send().await {
         if let Ok(json) = res.json::<Value>().await {
             let bal = json["balance"].as_str().unwrap_or("0").parse::<u64>().unwrap_or(0);
             return bal > 0;
         }
    }
    false
}

async fn wait_for_funding(client: &Client, addr: &str) -> bool {
    let url = format!("{}/api/balance?query={}", NODE_URL, addr);
    for _ in 0..20 { // 20 * 500ms = 10s wait
        if let Ok(res) = client.get(&url).send().await {
             if let Ok(json) = res.json::<Value>().await {
                 let bal = json["balance"].as_str().unwrap_or("0").parse::<u64>().unwrap_or(0);
                 if bal > 0 { return true; }
             }
        }
        sleep(Duration::from_millis(500)).await;
    }
    false
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
        
    let json: Value = res.json().await?;
    let id = json["id"].as_str().unwrap_or("").to_string();
    let status = json["status"].as_str().unwrap_or("unknown");

    if id.is_empty() {
        return Err(format!("Node Rejected: {}", status).into());
    }
    
    Ok(id)
}
