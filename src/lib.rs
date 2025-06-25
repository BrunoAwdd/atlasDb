// lib.rs

pub mod audit;
pub mod consensus;
pub mod cluster;
pub mod ffi;
pub mod network;
pub mod node;
pub mod storage;
pub mod utils;


// Reexporta os tipos principais para quem usar a lib
pub use node::{Graph, Vertex, Edge};
pub use cluster::{Cluster, ClusterNode};
pub use consensus::{ConsensusEngine, Proposal, Vote, ConsensusResult};
use serde_json::Value;
pub use storage::Storage;
pub use audit::{AuditData, save_audit, load_audit};
pub use utils::NodeId;

pub struct AtlasEnv {
    pub cluster: Cluster,
    pub graph: Graph,
    pub storage: Storage,
    pub engine: ConsensusEngine,
}

impl AtlasEnv {
    pub fn new(node_ids: &[&str]) -> Self {
        let mut cluster = Cluster::new();
        for id in node_ids {
            cluster.add_node((*id).into());
        }
        let nodes: Vec<NodeId> = cluster.nodes.keys().cloned().collect();

        AtlasEnv {
            cluster,
            graph: Graph::new(),
            storage: Storage::new(),
            engine: ConsensusEngine::new(nodes.len()),
        }
    }

    pub fn submit_json_proposal(&mut self, proposer: &str, json: Value) -> Proposal {
        let content = json.to_string();
        let proposal = self.engine.submit_proposal(proposer.into(), content);
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

    pub fn print(&self) {
        self.graph.print_graph();
        self.storage.print_summary();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;
    use serde_json::json;


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
    fn node_decide_vote(_node: &ClusterNode, _proposal: &Proposal) -> Vote {
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
    }
}
