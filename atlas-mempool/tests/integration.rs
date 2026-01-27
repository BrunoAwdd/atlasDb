use atlas_mempool::Mempool;
use atlas_common::transactions::{Transaction, SignedTransaction, signing_bytes};
use ed25519_dalek::{SigningKey, Signer};
use rand::rngs::OsRng;
use serial_test::serial;

fn mock_tx() -> SignedTransaction {
    let mut csprng = OsRng;
    let keypair = SigningKey::generate(&mut csprng);
    let verifying_key = keypair.verifying_key();
    let public_key = verifying_key.to_bytes().to_vec();
    // Derive address correctly using Bech32
    let from_address = atlas_common::address::address::Address::address_from_pk(&verifying_key, "nbex").unwrap();

    // Recipient Address
    let keypair_to = SigningKey::generate(&mut csprng);
    let verify_to = keypair_to.verifying_key();
    let to_address = atlas_common::address::address::Address::address_from_pk(&verify_to, "nbex").unwrap();

    let tx = Transaction {
        from: from_address,
        to: to_address,
        amount: 100,
        asset: "ATLAS".to_string(),
        nonce: 1,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        memo: None,
    };

    let msg = signing_bytes(&tx);
    let signature = keypair.sign(&msg).to_bytes().to_vec();

    SignedTransaction {
        transaction: tx,
        signature,
        public_key,
        fee_payer: None,
        fee_payer_signature: None,
        fee_payer_pk: None,
    }
}

#[tokio::test]
async fn test_local_mempool() {
    let mempool = Mempool::new(None).unwrap();
    let tx = mock_tx();

    assert!(mempool.add(tx.clone()).await.unwrap());
    assert_eq!(mempool.len().await.unwrap(), 1);
    
    let candidates = mempool.get_candidates(10).await.unwrap();
    assert_eq!(candidates.len(), 1);
}

#[tokio::test]
#[serial]
async fn test_redis_mempool_integration() {
    // Requires running Redis on localhost:6379 OR use testcontainers (skipped for simplicity here)
    // Run this test only if Redis is available.
    let redis_url = "redis://127.0.0.1/";
    let client = redis::Client::open(redis_url).unwrap();
    let mut con = client.get_multiplexed_async_connection().await;
    
    if con.is_err() {
        println!("⚠️ Redis not available, skipping test.");
        return;
    }
    let mut con = con.unwrap();
    let _: () = redis::AsyncCommands::flushall(&mut con).await.unwrap();

    let mempool = Mempool::new(Some(redis_url.to_string())).unwrap();
    let tx = mock_tx();

    assert!(mempool.add(tx.clone()).await.unwrap());
    assert_eq!(mempool.len().await.unwrap(), 1);

    let candidates = mempool.get_candidates(10).await.unwrap();
    assert_eq!(candidates.len(), 1);
    
    // Test Batched Removal
    let hash = candidates[0].0.clone();
    mempool.remove_batch(&[hash]).await.unwrap();
    assert_eq!(mempool.len().await.unwrap(), 0);
}
