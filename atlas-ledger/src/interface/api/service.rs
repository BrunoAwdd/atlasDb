use tonic::{Request, Response, Status};
// Import generated proto types. 
// Note: In an implementation, we usually `include!` the generated code in a `mod` block.
// For now, I'll refer to them assuming a specific module structure.

pub mod ledger_proto {
    tonic::include_proto!("ledger");
}

use ledger_proto::ledger_service_server::LedgerService;
use ledger_proto::{
    SubmitTransactionRequest, SubmitTransactionResponse,
    GetBalanceRequest, GetBalanceResponse,
    GetStatementRequest, GetStatementResponse,
    ListTransactionsRequest, ListTransactionsResponse
};
use tracing::{info, warn, error};
use std::sync::Arc;
use crate::Ledger;
use atlas_mempool::Mempool;

pub struct LedgerServiceImpl {
    pub ledger: Arc<Ledger>,
    pub mempool: Arc<Mempool>,
}

#[tonic::async_trait]
impl LedgerService for LedgerServiceImpl {
    async fn submit_transaction(
        &self,
        request: Request<SubmitTransactionRequest>,
    ) -> Result<Response<SubmitTransactionResponse>, Status> {
        let req = request.into_inner();
        info!("📝 [LedgerService] SubmitTransaction Request received from: {}", req.from);
        
        // validate inputs (basic)
        if req.amount.is_empty() {
            warn!("❌ [LedgerService] Amount empty");
            return Ok(Response::new(SubmitTransactionResponse {
                success: false,
                tx_hash: "".to_string(),
                error_message: "Amount required".to_string(),
            }));
        }

        // Construct Transaction
        let amount = req.amount.parse::<u128>().unwrap_or(0);
        let transaction = atlas_common::transactions::Transaction {
            from: req.from.clone(),
            to: req.to,
            amount,
            asset: req.asset,
            nonce: req.nonce,
            timestamp: req.timestamp,
            memo: req.memo,
        };

        // --- Security Verification ---
        // 1. Decode inputs
        let signature_bytes = match hex::decode(&req.signature) {
            Ok(b) => b,
            Err(e) => {
                warn!("❌ [LedgerService] Invalid Signature Hex: {}", e);
                return Err(Status::invalid_argument("Invalid signature hex"));
            }
        };
        let pk_bytes = match hex::decode(&req.public_key) {
            Ok(b) => b,
            Err(e) => {
                warn!("❌ [LedgerService] Invalid Public Key Hex: {}", e);
                return Err(Status::invalid_argument("Invalid public key hex"));
            }
        };

        if signature_bytes.len() != 64 {
            warn!("❌ [LedgerService] Invalid Signature Length: {}", signature_bytes.len());
            return Err(Status::invalid_argument("Invalid signature length"));
        }
        if pk_bytes.len() != 32 {
            warn!("❌ [LedgerService] Invalid Public Key Length: {}", pk_bytes.len());
            return Err(Status::invalid_argument("Invalid public key length"));
        }

        // 2. Verify Signature
        use ed25519_dalek::{Verifier, VerifyingKey, Signature};
        use atlas_common::transactions::signing_bytes;

        let verifying_key = VerifyingKey::from_bytes(pk_bytes.as_slice().try_into().unwrap())
            .map_err(|e| Status::invalid_argument(format!("Invalid public key: {}", e)))?;
        
        let signature = Signature::from_slice(&signature_bytes)
            .map_err(|e| Status::invalid_argument(format!("Invalid signature format: {}", e)))?;

        let msg = signing_bytes(&transaction);
        
        verifying_key.verify(&msg, &signature)
            .map_err(|_| {
                warn!("❌ [LedgerService] Signature Verification Failed at Ingestion! From: {}", req.from);
                Status::unauthenticated("Invalid signature")
            })?;

        // 3. Create SignedTransaction
        let signed_tx = atlas_common::transactions::SignedTransaction {
            transaction,
            signature: signature_bytes,
            public_key: pk_bytes,
        };

        // --- Idempotency Check ---
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(atlas_common::transactions::signing_bytes(&signed_tx.transaction));
        hasher.update(&signed_tx.signature);
        let hash = hex::encode(hasher.finalize());

        if self.ledger.exists_transaction(&hash).await.unwrap_or(false) {
            info!("♻️ [LedgerService] Transaction already exists in Ledger: {}", hash);
            return Ok(Response::new(SubmitTransactionResponse {
                success: true, // It is successful in the sense that it's processed (idempotent success)
                tx_hash: hash,
                error_message: "Transaction already confirmed (Idempotent)".to_string(),
            }));
        }

        // Add to Mempool
        let new = match self.mempool.add(signed_tx) {
            Ok(n) => {
                info!("✅ [LedgerService] Transaction added to mempool. New? {}", n);
                n
            },
            Err(e) => {
                 error!("❌ [LedgerService] Mempool Validation Failed: {}", e);
                 return Err(Status::invalid_argument(format!("Mempool validation failed: {}", e)));
            }
        };

        // TODO: Compute hash to return it
        // For now returning "pending" or a placeholder hash
        let tx_hash = "pending-hash".to_string(); // Ideally compute real hash

        Ok(Response::new(SubmitTransactionResponse {
            success: true,
            tx_hash,
            error_message: if new { "".to_string() } else { "Transaction already in mempool".to_string() },
        }))
    }

    async fn get_balance(
        &self,
        request: Request<GetBalanceRequest>,
    ) -> Result<Response<GetBalanceResponse>, Status> {
        let req = request.into_inner();
        let state = self.ledger.state.read().await;
        
        // Handle address prefix if missing
        let address = if req.address.starts_with("passivo:wallet:") {
            req.address.clone()
        } else {
            format!("passivo:wallet:{}", req.address)
        };

        // Lookup account
        let balance = if let Some(account) = state.accounts.get(&address) {
            account.get_balance(&req.asset).to_string()
        } else {
            "0".to_string()
        };

        Ok(Response::new(GetBalanceResponse {
            balance,
            asset: req.asset,
        }))
    }

    async fn get_statement(
        &self,
        request: Request<GetStatementRequest>,
    ) -> Result<Response<GetStatementResponse>, Status> {
       let req = request.into_inner();
       let proposals = self.ledger.get_all_proposals().await.map_err(|e| Status::internal(e.to_string()))?;

       let mut records = Vec::new();
       
       for p in proposals {
           // Try parsing the content as a SignedTransaction
           if let Ok(signed_tx) = serde_json::from_str::<atlas_common::transactions::SignedTransaction>(&p.content) {
               let tx = signed_tx.transaction;
               
               // Filter: Check if address matches From or To
               // Ledger uses "passivo:wallet:addr", but user might send just "addr" or the full path.
               // We should check if the transaction involves the requested address.
               // The tx.from/to might be bare addresses or full paths depending on how SubmitTransaction sent them.
               // In SubmitTransaction we used `req.from` directly.
               
               // Normalize check: contains substring?
               if tx.from.contains(&req.address) || tx.to.contains(&req.address) {
                   records.push(ledger_proto::TransactionRecord {
                       tx_hash: p.hash, // Proposal hash is the tx hash
                       from: tx.from,
                       to: tx.to,
                       amount: tx.amount.to_string(),
                       asset: tx.asset,
                       timestamp: p.time as u64,
                       memo: tx.memo.unwrap_or_default(),
                   });
               }
           } else if let Ok(tx) = serde_json::from_str::<atlas_common::transactions::Transaction>(&p.content) {
               // FALLBACK: Support legacy unsigned transactions (for previous blocks or simulation)
               if tx.from.contains(&req.address) || tx.to.contains(&req.address) {
                   records.push(ledger_proto::TransactionRecord {
                       tx_hash: p.hash, // Proposal hash is the tx hash
                       from: tx.from,
                       to: tx.to,
                       amount: tx.amount.to_string(),
                       asset: tx.asset,
                       timestamp: p.time as u64,
                       memo: tx.memo.unwrap_or_default(),
                   });
               }
           }
       }
       
       // Sort by timestamp desc
       records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

       Ok(Response::new(GetStatementResponse {
           transactions: records
       }))
    }

    async fn list_transactions(
        &self,
        request: Request<ListTransactionsRequest>,
    ) -> Result<Response<ListTransactionsResponse>, Status> {
       let req = request.into_inner();
       let proposals = self.ledger.get_all_proposals().await.map_err(|e| Status::internal(e.to_string()))?;

       let mut records = Vec::new();
       
       for p in proposals {
            let tx_res = if let Ok(signed_tx) = serde_json::from_str::<atlas_common::transactions::SignedTransaction>(&p.content) {
                Some(signed_tx.transaction)
            } else if let Ok(tx) = serde_json::from_str::<atlas_common::transactions::Transaction>(&p.content) {
                Some(tx)
            } else {
                None
            };

            if let Some(tx) = tx_res {
               records.push(ledger_proto::TransactionRecord {
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
       
       // Sort by timestamp desc
       records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

       let total_count = records.len() as u64;
       
       // Pagination
       let skip = req.offset as usize;
       let take = if req.limit == 0 { 50 } else { req.limit as usize };
       
       let paged = records.into_iter().skip(skip).take(take).collect();

       Ok(Response::new(ListTransactionsResponse {
           transactions: paged,
           total_count,
       }))
    }
}
