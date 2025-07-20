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


}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use crate::{
        cluster::node::Node,
        env::consensus::Vote
    };

    /* 

    /// Simulation of a voting process in the Atlas environment.
    fn simulate_voting(env: &mut AtlasEnv, proposal: &Proposal) {
        for node in env.cluster.nodes.values() {
            let vote = node_decide_vote(&node, proposal);
            env.storage
                .log_vote(&proposal.id, node.id.clone(), vote.clone());
            env.engine
                .receive_vote(&proposal.id, node.id.clone(), vote);
        }
    }

    /// Simulated decision logic for a node's vote on a proposal.
    ///
    /// 90% chance to vote YES, 10% NO — mimicking stochastic consensus dynamics.
    fn node_decide_vote(_node: &Node, _proposal: &Proposal) -> Vote {
        if rand::thread_rng().gen_bool(0.9) {
            Vote::Yes
        } else {
            Vote::No
        }
    }


    
    #[test]
    fn test_simple_proposal_flow() {
        // 🧪 Cria um ambiente com 5 nós
        let mut env = AtlasEnv::new(&["nó-A", "nó-B", "nó-C", "nó-D", "nó-E"]);

        // ➕ Adiciona vértices básicos
        env.graph.add_vertex(Vertex::new("A", "Person"));
        env.graph.add_vertex(Vertex::new("B", "Place"));

        // 📤 Submete proposta
        let json = json!({
            "action": "add_edge",
            "from": "A",
            "to": "B",
            "label": "visits"
        });

        let proposal = env.submit_json_proposal("nó-A", json);

        // 🗳️ Votação simulada
        simulate_voting(&mut env, &proposal);

        // 📊 Avaliação
        let results = env.evaluate_all();
        assert_eq!(results.len(), 1);

        let (_, result) = &results[0];
        assert_eq!(result.proposal_id, proposal.id);

        // ✅ Aplicação se aprovada
        env.apply_if_approved(&proposal, result);

        // 📌 Verificação de resultado
        let edge_found = env
            .graph
            .edges
            .iter()
            .any(|e| e.from == "A" && e.to == "B" && e.label == "visits");

        if result.approved {
            assert!(edge_found, "Edge should be added if approved.");
        } else {
            assert!(!edge_found, "Edge should not be added if rejected.");
        }
    }*/
}
