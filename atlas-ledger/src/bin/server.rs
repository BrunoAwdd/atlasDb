use tonic::transport::Server;
use atlas_ledger::interface::api::service::{LedgerServiceImpl, ledger_proto::ledger_service_server::LedgerServiceServer};
use atlas_ledger::Ledger;
use std::sync::Arc;
use tower_http::cors::{CorsLayer, Any};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;
    println!("⚠️  DEPRECATED: This standalone server is deprecated. Please use `atlas-node` instead.");
    println!("Atlas Ledger Server listening on {}", addr);

    // Initialize Ledger with local path for consistency with atlas-node
    let data_dir = "data/db";
    println!("Using data directory: {}", data_dir);
    let ledger = Ledger::new(data_dir).await?;
    let ledger = Arc::new(ledger);
    let mempool = Arc::new(atlas_mempool::Mempool::new());

    let service = LedgerServiceImpl { ledger: ledger.clone(), mempool };
    
    // Enable gRPC-Web and CORS
    let service = tonic_web::enable(LedgerServiceServer::new(service));
    
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    // Start REST API (Axum)
    let ledger_for_api = ledger.clone();
    tokio::spawn(async move {
        use axum::{routing::get, Router, extract::{State, Query}, Json};
        use serde::{Deserialize, Serialize};
        
        #[derive(Deserialize)]
        struct ListParams {
            limit: Option<usize>,
            offset: Option<usize>, // support offset
            query: Option<String>,
        }

        #[derive(Serialize)]
        struct TxDto {
            tx_hash: String,
            from: String,
            to: String,
            amount: String,
            asset: String,
            timestamp: u64,
            memo: String,
        }

        #[derive(Serialize)]
        struct ListResponse {
            transactions: Vec<TxDto>,
            total_count: u64,
        }

        async fn list_transactions_api(
            State(ledger): State<Arc<Ledger>>,
            Query(params): Query<ListParams>,
        ) -> Json<ListResponse> {
            let proposals = match ledger.get_all_proposals().await {
                 Ok(p) => p,
                 Err(_) => return Json(ListResponse { transactions: vec![], total_count: 0 }),
            };
            
            let mut records = Vec::new();
            let query = params.query.as_deref().unwrap_or("").to_lowercase();

            for p in proposals {
                 let tx_res = if let Ok(signed_tx) = serde_json::from_str::<atlas_common::transactions::SignedTransaction>(&p.content) {
                     Some(signed_tx.transaction)
                 } else if let Ok(tx) = serde_json::from_str::<atlas_common::transactions::Transaction>(&p.content) {
                     Some(tx)
                 } else {
                     None
                 };

                 if let Some(tx) = tx_res {
                    // Filter Logic
                    if !query.is_empty() {
                         let match_hash = p.hash.to_lowercase().contains(&query);
                         let match_from = tx.from.to_lowercase().contains(&query);
                         let match_to = tx.to.to_lowercase().contains(&query);
                         if !match_hash && !match_from && !match_to {
                             continue;
                         }
                    }

                    records.push(TxDto {
                            tx_hash: p.hash,
                            from: tx.from,
                            to: tx.to,
                            amount: tx.amount.to_string(),
                            asset: tx.asset,
                            timestamp: p.time as u64,
                            memo: tx.memo.unwrap_or_default(),
                    });
                 }
            }
            records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            
            let total_count = records.len() as u64;
            let skip = params.offset.unwrap_or(0);
            let take = params.limit.unwrap_or(50);
            
            let paged = records.into_iter().skip(skip).take(take).collect();
            
            Json(ListResponse { transactions: paged, total_count })
        }

        let app = Router::new()
            .route("/api/transactions", get(list_transactions_api))
            .with_state(ledger_for_api)
            .layer(tower_http_axum::cors::CorsLayer::permissive());

        println!("REST API listening on 0.0.0.0:3001");
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
        axum::serve(listener, app).await.unwrap();
    });

    Server::builder()
        .accept_http1(true) // Required for gRPC-Web
        .layer(cors)
        .add_service(service) // Wrapped service
        .serve(addr)
        .await?;

    Ok(())
}
