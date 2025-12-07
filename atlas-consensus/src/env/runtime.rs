use std::{
    collections::{HashMap, HashSet},
    sync::Arc
};
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn};

use atlas_common::env::{
    consensus::types::ConsensusResult,
    node::Graph,
    proposal::Proposal,
};
use atlas_common::utils::NodeId;
use atlas_p2p::PeerManager;
use atlas_ledger::storage::Storage;
use crate::ConsensusEngine;

// Callback type alias
pub type Callback = Arc<dyn Fn(ConsensusResult) + Send + Sync>;

pub struct AtlasEnv {
    pub graph: Graph,
    pub storage: Arc<RwLock<Storage>>,
    pub engine: Arc<Mutex<ConsensusEngine>>,

    pub callback: Callback, // Updated to use the new Callback type alias

    pub peer_manager: Arc<RwLock<PeerManager>>,
    pub data_dir: String,
}

use crate::QuorumPolicy;
use atlas_common::env::node::Edge;
use atlas_ledger::storage::audit::save_audit;

impl AtlasEnv {
    pub fn new(
        callback: Callback,
        peer_manager: Arc<RwLock<PeerManager>>,
        data_dir: &str,
    ) -> Self {
        let policy = QuorumPolicy {
            fraction: 0.7,
            min_voters: 1,
        };
        let engine = ConsensusEngine::new(Arc::clone(&peer_manager), policy);
        AtlasEnv {
            graph: Graph::new(),
            storage: Arc::new(RwLock::new(Storage::new(data_dir))),
            engine: Arc::new(Mutex::new(engine)),
            callback,
            peer_manager,
            data_dir: data_dir.to_string(),
        }
    }
    
    pub async fn evaluate_all(&mut self) -> Result<Vec<(String, ConsensusResult)>, String> {
        let result = self.engine
            .lock()
            .await
            .evaluate_proposals()
            .await
            .into_iter()
            .map(|res| {
                res
            })
            .collect::<Vec<_>>();

        for res in &result {
             self.storage.write().await.log_result(&res.proposal_id, res.clone());
        }

        Ok(result.into_iter().map(|r| (r.proposal_id.clone(), r)).collect())
    }

    pub fn apply_if_approved(&mut self, proposal: &Proposal, result: &ConsensusResult) {
        if result.approved {
            if let Ok(data) = serde_json::from_str::<Value>(&proposal.content) {
                if data["action"] == "add_edge" {
                    let from = data["from"].as_str().unwrap_or("");
                    let to = data["to"].as_str().unwrap_or("");
                    let label = data["label"].as_str().unwrap_or("related_to");

                    self.graph.add_edge(Edge::new(from, to, label));
                    info!(
                        "✅ Edge added to graph: [{}] --{}--> [{}]",
                        from, label, to
                    );
                }
            }
        } else {
            info!("❌ Proposal rejected — graph remains unchanged.");
        }
    }

    pub async fn export_audit(&self, path: &str) {
        let audit = self.storage.read().await.to_audit();
        if let Err(err) = save_audit(path, &audit) {
            warn!("Warning: failed to export audit data to {}: {}", path, err);
        }
    }

    pub async fn get_nodes(&self) -> HashSet<NodeId> {
        self.peer_manager.read()
            .await
            .get_active_peers()
    }

    pub async fn print(&self) {
        self.graph.print_graph();
        self.storage.read().await.print_summary();
    }

    pub async fn get_proposals(&self) -> Result<HashMap<String, Proposal>, String> {
        let proposals = self.engine.lock().await.pool.all().clone();

        Ok(proposals)
    }
}