pub mod consensus;
pub mod config;
pub mod node;
pub mod storage;
pub mod proposal;
pub mod vote_data;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc
};

use serde_json::Value;

use tokio::sync::{Mutex, RwLock};

use crate::{
    peer_manager::PeerManager, 
    utils::NodeId
};

use consensus::{ConsensusEngine, ConsensusResult};
use proposal::Proposal;
use node::{Graph, Edge};
use storage::{Storage, audit::save_audit};

pub trait Callback: Fn(ConsensusResult) + Send + Sync {}
impl<T> Callback for T where T: Fn(ConsensusResult) + Send + Sync {}

pub struct AtlasEnv {
    pub graph: Graph,
    pub storage: Storage,
    pub engine: Arc<Mutex<ConsensusEngine>>,

    pub callback: Arc<dyn Callback>,

    pub peer_manager: Arc<RwLock<PeerManager>>,
}

impl AtlasEnv {
    pub fn new(
        callback:  Arc<dyn Callback>,
        peer_manager: Arc<RwLock<PeerManager>>,
    ) -> Self {
        let engine = ConsensusEngine::new(Arc::clone(&peer_manager), 70.0);
        AtlasEnv {
            graph: Graph::new(),
            storage: Storage::new(),
            engine: Arc::new(Mutex::new(engine)),
            callback,
            peer_manager,
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
                self.storage
                    .log_result(&res.proposal_id, res.clone());
                (res.proposal_id.clone(), res)
            })
            .collect();

        Ok(result)
    }

    pub fn apply_if_approved(&mut self, proposal: &Proposal, result: &ConsensusResult) {
        if result.approved {
            if let Ok(data) = serde_json::from_str::<Value>(&proposal.content) {
                if data["action"] == "add_edge" {
                    let from = data["from"].as_str().unwrap_or("");
                    let to = data["to"].as_str().unwrap_or("");
                    let label = data["label"].as_str().unwrap_or("related_to");

                    self.graph.add_edge(Edge::new(from, to, label));
                    println!(
                        "✅ Edge added to graph: [{}] --{}--> [{}]",
                        from, label, to
                    );
                }
            }
        } else {
            println!("❌ Proposal rejected — graph remains unchanged.");
        }
    }

    pub fn export_audit(&self, path: &str) {
        let audit = self.storage.to_audit();
        if let Err(err) = save_audit(path, &audit) {
            eprintln!("Warning: failed to export audit data to {}: {}", path, err);
        }
    }

    pub async fn get_nodes(&self) -> HashSet<NodeId> {
        self.peer_manager.read()
            .await
            .get_active_peers()
    }

    pub fn print(&self) {
        self.graph.print_graph();
        self.storage.print_summary();
    }

    pub async fn get_proposals(&self) -> Result<HashMap<String, Proposal>, String> {
        let proposals = self.engine.lock().await.pool.all().clone();

        Ok(proposals)
    }
}