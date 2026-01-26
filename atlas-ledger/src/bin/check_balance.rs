use atlas_ledger::interface::api::service::ledger_proto::ledger_service_client::LedgerServiceClient;
use atlas_ledger::interface::api::service::ledger_proto::GetBalanceRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: check_balance <ADDRESS> [ASSET] [PORT]");
        println!("Example: check_balance nbex... ATLAS 50051");
        return Ok(());
    }

    let address = args[1].clone();
    let asset = args.get(2).unwrap_or(&"ATLAS".to_string()).clone();
    let port = args.get(3).unwrap_or(&"50051".to_string()).clone();
    let addr = format!("http://0.0.0.0:{}", port);

    println!("Connecting to {}...", addr);
    let mut client = LedgerServiceClient::connect(addr).await?;

    let req = GetBalanceRequest {
        address: address.clone(),
        asset: asset.clone(),
    };

    let resp = client.get_balance(req).await?;
    let balance = resp.into_inner();

    println!("💰 Balance for {}:", address);
    println!("   Asset:   {}", balance.asset);
    println!("   Amount:  {}", balance.balance);
    println!("   Nonce:   {}", balance.nonce);

    Ok(())
}
