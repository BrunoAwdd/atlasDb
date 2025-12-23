use std::sync::Arc;
use tracing::{info, warn, error};
use atlas_p2p::{ports::P2pPublisher, protocol::{TxRequest, TxBundle}};
use atlas_consensus::cluster::core::Cluster;
use atlas_common::utils::NodeId;

pub struct SyncDriver<P: P2pPublisher> {
    cluster: Arc<Cluster>,
    p2p: P,
}

impl<P: P2pPublisher> SyncDriver<P> {
    pub fn new(cluster: Arc<Cluster>, p2p: P) -> Self {
        Self { cluster, p2p }
    }

    pub async fn handle_tx_request(&self, from: NodeId, req: TxRequest, req_id: u64) {
        match req {
            TxRequest::GetState { height } => {
                info!("📥 Received state request from {} (height > {})", from, height);
                let proposals = self.cluster.local_env.storage.read().await.get_proposals_after(height).await;
                let bundle = TxBundle::State { proposals };
                
                // Send response
                if let Err(e) = self.p2p.send_response(req_id, bundle).await {
                    error!("Failed to send state response: {}", e);
                }
            },
            _ => {}
        }
    }

    pub async fn handle_tx_bundle(&self, from: NodeId, bundle: TxBundle) {
        match bundle {
            TxBundle::State { proposals } => {
                info!("📦 Received state bundle from {} with {} proposals", from, proposals.len());
                for p in proposals {
                    // Verify signature
                    let sign_bytes = atlas_common::env::proposal::signing_bytes(&p);
                    let ok = self.cluster.auth.read().await
                        .verify_with_key(sign_bytes, &p.signature, &p.public_key)
                        .is_ok();

                    if ok {
                        self.cluster.local_env.storage.write().await.log_proposal(p).await;
                    } else {
                        warn!("❌ Invalid signature in State Transfer for proposal {}", p.id);
                    }
                }
            },
            _ => {}
        }
    }
}
