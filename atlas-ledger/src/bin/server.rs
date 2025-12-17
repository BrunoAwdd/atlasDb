use tonic::transport::Server;
use atlas_ledger::interface::api::service::{LedgerServiceImpl, ledger_proto::ledger_service_server::LedgerServiceServer};
use atlas_ledger::Ledger;
use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;
    println!("Atlas Ledger Server listening on {}", addr);

    // Initialize Ledger with a temp path for simulation
    let data_dir = "/tmp/atlas_simulation_ledger";
    let ledger = Ledger::new(data_dir).await?;
    let ledger = Arc::new(ledger);
    let mempool = Arc::new(atlas_mempool::Mempool::new());

    let service = LedgerServiceImpl { ledger, mempool };
    
    // Enable gRPC-Web and CORS
    let service = tonic_web::enable(LedgerServiceServer::new(service));
    
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    Server::builder()
        .accept_http1(true) // Required for gRPC-Web
        .layer(cors)
        .add_service(service) // Wrapped service
        .serve(addr)
        .await?;

    Ok(())
}
