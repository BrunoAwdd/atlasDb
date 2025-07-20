pub mod consensus;
pub mod config;
pub mod node;
pub mod storage;
pub mod proposal;

use std::{
    collections::HashSet,
    sync::{Arc, RwLock}, 
    time::{SystemTime, UNIX_EPOCH}
};

use serde_json::Value;

use crate::{
    cluster_proto::Ack, 
    env::config::EnvConfig, 
    network::adapter::NetworkAdapter, 
    peer_manager::PeerManager, 
    utils::NodeId
};

use consensus::{ConsensusEngine, ConsensusResult};
use proposal::Proposal;
use node::{Graph, Edge};
use storage::{Storage, audit::save_audit};

pub trait Callback: Fn(ConsensusResult) + Send + Sync {}
impl<T> Callback for T where T: Fn(ConsensusResult) + Send + Sync {}


#[derive( Clone)]
pub struct AtlasEnv {
    pub graph: Graph,
    pub storage: Storage,
    pub engine: ConsensusEngine,

    pub network: Arc<RwLock<dyn NetworkAdapter>>,

    //pub auth: Arc<dyn Authenticator>,

    pub callback: Arc<dyn Callback>,

    pub peer_manager: Arc<RwLock<PeerManager>>,
}

impl AtlasEnv {
    pub fn new(
        network: Arc<RwLock<dyn NetworkAdapter>>, 
        callback:  Arc<dyn Callback>,
        peer_manager: Arc<RwLock<PeerManager>>,
        path: Option<&str>,
    ) -> Self {
        let env = AtlasEnv {
            graph: Graph::new(),
            storage: Storage::new(),
            engine: ConsensusEngine::new(Arc::clone(&peer_manager), 70.0),
            network,
            callback,
            peer_manager,
        };

        env.save_config(path.unwrap_or("config.json"))
            .expect("Failed to save initial configuration");

        env
    }

    pub fn submit_json_proposal(&mut self, proposer: NodeId, json: Value) -> Proposal {
        let content = json.to_string();
        let proposal = self.engine.submit_proposal(proposer, content, None);
        self.storage.log_proposal(proposal.clone());
        proposal
    }

    pub fn evaluate_all(&mut self) -> Vec<(String, ConsensusResult)> {
        self.engine
            .evaluate_proposals()
            .into_iter()
            .map(|res| {
                self.storage
                    .log_result(&res.proposal_id, res.clone());
                (res.proposal_id.clone(), res)
            })
            .collect()
    }
    pub async fn submit_proposal(&mut self, proposal: &Proposal, node_id: NodeId) -> Result<Ack, String> {
        self.engine
            .submit_proposal(
                proposal.clone(), 
                Arc::clone(&self.network), 
                node_id
            )
            .await
    }

    pub async fn send_heartbeat(&self, node: crate::cluster::node::Node, to: &NodeId) -> Result<Ack, String> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "Failed to get system time")?
            .as_secs();

        let peer = {
            let manager = self.peer_manager.read()
                .map_err(|_| "Failed to acquire read lock on peer manager")?;
            manager.get_peer_stats(&to)
                .ok_or_else(|| format!("Peer {} not found", to.0))?
        };

        let heartbeat_msg = format!("{}: heartbeat from {}", peer.address, node.id);

        // Send heartbeat with error handling
        let network = self.network.write()
            .map_err(|_| "Failed to acquire write lock on network adapter")?;
            
        network.send_heartbeat(
            node.id.clone(), 
            peer.clone(), 
            heartbeat_msg.clone()
        ).await
        .map_err(|e| format!("Network error: {:?}", e))?;

        Ok(Ack {
            received: true,
            message: format!("✅ Heartbeat enviado com sucesso para {}", to),
        })
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

    pub fn get_nodes(&self) -> HashSet<NodeId> {
        self.peer_manager.read()
            .expect("Failed to acquire read lock")
            .get_active_peers()
    }

    pub fn print(&self) {
        self.graph.print_graph();
        self.storage.print_summary();
    }

    pub fn from_config(network: Arc<RwLock<dyn NetworkAdapter>>) -> Self {
        let config = EnvConfig::load_from_file("config.json").expect("Failed to load config file");
        config.build_env(network)
    }

    pub fn save_config(&self, path: &str) -> std::io::Result<()> {
        let config = EnvConfig::new(
            self.graph.clone(),
            self.storage.clone(),
            self.peer_manager.read().unwrap().clone(),
            self.engine.quorum_ratio,
            self.engine.proposals.clone(),
            self.engine.votes.clone(),
        );
        config.save_to_file(path)
    }

    pub fn get_proposals(&self) -> Vec<Proposal> {
        let proposals = self.engine.proposals.clone();

        proposals
    }
        }
    }*/
}
