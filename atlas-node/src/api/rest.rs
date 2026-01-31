use std::sync::Arc;
use std::collections::HashMap;
use tokio::net::TcpListener;
use axum::{
    routing::{get, post},
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
struct AccountView {
    r#type: String, // "user", "system", "issuance"
    assets: HashMap<String, String>,
    liabilities: HashMap<String, String>,
    equity: HashMap<String, String>,
}

#[derive(Serialize)]
struct BalanceResponse {
    address: String,
    asset: String,
    balance: String,
    balances: HashMap<String, String>, // Full Portfolio
    nonce: u64,
    view: AccountView,
}

#[derive(Deserialize)]
struct CreateTxRequest {
    transaction: atlas_common::transactions::Transaction,
    signature: Vec<u8>,
    public_key: Vec<u8>,
    fee_payer: Option<String>,
    fee_payer_signature: Option<Vec<u8>>,
    fee_payer_pk: Option<Vec<u8>>,
}

#[derive(Serialize)]
struct CreateTxResponse {
    id: String,
    status: String,
}

pub async fn start_rest_api(port: u16, state: AppState) {
    let app = Router::new()
        .route("/api/transactions", get(list_transactions_api))
        .route("/api/transaction", post(create_transaction_api))
        .route("/api/mempool", get(list_mempool_api))
        .route("/api/balance", get(get_balance_api))
        .route("/api/accounts", get(list_accounts_api))
        .route("/api/tokens", get(list_tokens_api))
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
    let raw_address = params.query.unwrap_or_default();
    // Normalize address lookup
    let address = if raw_address.contains(':') {
        raw_address.clone()
    } else {
        format!("wallet:{}", raw_address)
    };

    let asset = atlas_ledger::core::ledger::asset::ATLAS_FULL_ID;
    
    // Fetch full account state to get nonce + balance
    let account = state.ledger.get_account(&address).await.unwrap_or_default();
    let balance = account.get_balance(&asset.to_string());
    
    // Convert all balances to string map
    let all_balances: HashMap<String, String> = account.balances.iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();

    // --- Backend Accounting Classification ---
    let mut view = AccountView {
        r#type: "user".to_string(),
        assets: HashMap::new(),
        liabilities: HashMap::new(),
        equity: HashMap::new(),
    };

    if address == "vault:issuance" {
        view.r#type = "issuance".to_string();
        // Issuance Vault: Holds Authorized Capital.
        // In our model, this is Equity Source.
        // We map it to EQUITY.
        for (k, v) in &all_balances {
            view.equity.insert(k.clone(), v.clone());
        }
    } else if address.starts_with("vault:") || address.starts_with("wallet:mint") {
        view.r#type = "system".to_string();
        // System Vaults/Mint: Holds unissued assets or reserves.
        // Treated as ASSETS.
        for (k, v) in &all_balances {
            view.assets.insert(k.clone(), v.clone());
        }
    } else {
        view.r#type = "user".to_string();
        // User Wallet:
        // 1. All holdings are ASSETS (User perspective).
        // 2. We apply Double Entry: Assets = Equity (Net Worth).
        for (k, v) in &all_balances {
             view.assets.insert(k.clone(), v.clone());
             
             // Mirror to Equity (Net Worth)
             // We can aggregate or just mirror item by item?
             // Inspector expects items. Let's mirror.
             view.equity.insert(k.clone(), v.clone());
        }
    }

    Json(BalanceResponse {
        address, // Return the normalized address
        asset: asset.to_string(),
        balance: balance.to_string(),
        balances: all_balances,
        nonce: account.nonce,
        view,
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
     let raw_query = params.query.as_deref().unwrap_or("").to_lowercase();
     // Strip prefix if present to match raw addresses in txs
     let query = raw_query.strip_prefix("wallet:").unwrap_or(&raw_query);
     
     // info!("API: listing transactions query='{}' proposals={}", query, proposals.len());
     
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
                 let match_hash = p.hash.to_lowercase().contains(query);
                 let match_from = tx.from.to_lowercase().contains(query);
                 let match_to = tx.to.to_lowercase().contains(query);
                 
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
    match state.mempool.get_all().await {
        Ok(txs) => Json(txs),
        Err(e) => {
            tracing::error!("Failed to fetch mempool: {}", e);
            Json(vec![])
        }
    }
}

async fn list_accounts_api(
    State(state): State<AppState>,
) -> Json<HashMap<String, atlas_ledger::core::ledger::account::AccountState>> {
   Json(state.ledger.get_all_accounts().await)
}

async fn list_tokens_api(
    State(state): State<AppState>,
) -> Json<HashMap<String, atlas_ledger::core::ledger::asset::AssetDefinition>> {
   Json(state.ledger.get_all_assets().await)
}

async fn create_transaction_api(
    State(state): State<AppState>,
    Json(payload): Json<CreateTxRequest>,
) -> Json<CreateTxResponse> {
    let st = atlas_common::transactions::SignedTransaction {
        transaction: payload.transaction,
        signature: payload.signature,
        public_key: payload.public_key,
        fee_payer: payload.fee_payer,
        fee_payer_signature: payload.fee_payer_signature,
        fee_payer_pk: payload.fee_payer_pk,
    };
    
    match state.mempool.add(st).await {
        Ok(id) => Json(CreateTxResponse { id, status: "accepted".to_string() }),
        Err(e) => Json(CreateTxResponse { id: "".to_string(), status: format!("rejected: {}", e) }),
    }
}
