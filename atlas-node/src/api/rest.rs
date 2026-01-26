use std::sync::Arc;
use std::collections::HashMap;
use tokio::net::TcpListener;
use axum::{
    routing::get,
    Router,
    extract::{State, Query},
    Json
};
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Clone)]
pub struct AppState {
    pub ledger: Arc<atlas_ledger::Ledger>,
    pub mempool: Arc<atlas_mempool::Mempool>,
}

#[derive(Deserialize)]
struct ListParams {
    limit: Option<usize>,
    offset: Option<usize>,
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
    fee_payer: Option<String>,
}

#[derive(Serialize)]
struct ListResponse {
    transactions: Vec<TxDto>,
    total_count: u64,
}

#[derive(Serialize)]
struct BalanceResponse {
    address: String,
    asset: String,
    balance: String,
}

pub async fn start_rest_api(port: u16, state: AppState) {
    let app = Router::new()
        .route("/api/transactions", get(list_transactions_api))
        .route("/api/mempool", get(list_mempool_api))
        .route("/api/balance", get(get_balance_api))
        .route("/api/accounts", get(list_accounts_api))
        .with_state(state)
        .layer(tower_http_axum::cors::CorsLayer::permissive());

    info!("REST API listening on 0.0.0.0:{}", port);
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_balance_api(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Json<BalanceResponse> {
    let address = params.query.unwrap_or_default();
    let asset = "ATLAS";
    // Ledger::get_balance
    let balance = state.ledger.get_balance(&address, asset).await.unwrap_or(0);
    Json(BalanceResponse {
        address,
        asset: asset.to_string(),
        balance: balance.to_string(),
    })
}

async fn list_transactions_api(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Json<ListResponse> {
     let proposals = match state.ledger.get_all_proposals().await {
         Ok(p) => p,
         Err(_) => return Json(ListResponse { transactions: vec![], total_count: 0 }),
     };
     
     let mut records = Vec::new();
     let query = params.query.as_deref().unwrap_or("").to_lowercase();
     
     for p in proposals {
         // Try Batch first (Standard)
         let mut tx_list = Vec::new();
         
         if let Ok(batch) = serde_json::from_str::<Vec<atlas_common::transactions::SignedTransaction>>(&p.content) {
             for st in batch {
                 tx_list.push((st.transaction, st.fee_payer));
             }
         } else if let Ok(signed_tx) = serde_json::from_str::<atlas_common::transactions::SignedTransaction>(&p.content) {
             tx_list.push((signed_tx.transaction, signed_tx.fee_payer));
         } else if let Ok(tx) = serde_json::from_str::<atlas_common::transactions::Transaction>(&p.content) {
             tx_list.push((tx, None));
         }

         for (tx, fee_payer) in tx_list {

            if !query.is_empty() {
                 let match_hash = p.hash.to_lowercase().contains(&query);
                 let match_from = tx.from.to_lowercase().contains(&query);
                 let match_to = tx.to.to_lowercase().contains(&query);
                 if !match_hash && !match_from && !match_to {
                     continue;
                 }
            }
            records.push(TxDto {
                    tx_hash: p.hash.clone(),
                    from: tx.from,
                    to: tx.to,
                    amount: tx.amount.to_string(),
                    asset: tx.asset,
                    timestamp: p.time as u64,
                    memo: tx.memo.unwrap_or_default(),
                    fee_payer,
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

async fn list_mempool_api(
    State(state): State<AppState>,
) -> Json<Vec<String>> {
    Json(state.mempool.get_all())
}

async fn list_accounts_api(
    State(state): State<AppState>,
) -> Json<HashMap<String, atlas_ledger::core::ledger::account::AccountState>> {
   Json(state.ledger.get_all_accounts().await)
}
