
pub mod identity;
pub mod session;
pub mod profile;
pub mod vault;
pub mod errors;
pub mod wasm;
pub mod wallet;

#[cfg(feature = "grpc")]
pub mod ledger_proto {
    tonic::include_proto!("ledger");
}

#[cfg(feature = "grpc")]
use ledger_proto::ledger_service_client::LedgerServiceClient;
#[cfg(feature = "grpc")]
use ledger_proto::SubmitTransactionRequest;

#[cfg(feature = "grpc")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Mel Blockchain - Wallet CLI");

    // Connect to Ledger
    let mut client = LedgerServiceClient::connect("http://127.0.0.1:50051").await?;
    println!("✅ Connected to Ledger at 127.0.0.1:50051");

    // Simulate sending money
    let request = tonic::Request::new(SubmitTransactionRequest {
        from: "alice".into(),
        to: "bob".into(),
        amount: "500".into(),
        asset: "USD".into(),
        memo: Some("Paying for lunch".into()),
        signature: "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000".into(),
        public_key: "0000000000000000000000000000000000000000000000000000000000000000".into(),
    });

    let response = client.submit_transaction(request).await?;
    println!("RESPONSE={:?}", response.into_inner());

    Ok(())
}

#[cfg(not(feature = "grpc"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Mel Blockchain - Wallet CLI");
    println!("⚠️ gRPC feature not enabled. Run with --features grpc to use the ledger client.");
    Ok(())
}