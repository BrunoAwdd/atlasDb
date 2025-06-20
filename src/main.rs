pub mod node;
pub mod cluster;
pub mod utils;
pub mod consensus;
pub mod storage;

use consensus::{ConsensusEngine, Vote, Proposal};
use rand::Rng;
use utils::NodeId;
use storage::Storage;
use node::{Graph, Vertex, Edge};
use serde_json::{Value};
use cluster::{Cluster, ClusterNode};

/// Entry point of the AtlasDB simulation.
///
/// Simulates the lifecycle of a distributed graph mutation:
/// - Initializing a cluster
/// - Creating a proposal
/// - Running distributed voting
/// - Evaluating consensus
/// - Applying mutation if approved
fn main() {
    // 🧠 Initialize a simulated cluster with 5 nodes
    let mut cluster = Cluster::new();
    for id in ["nó-A", "nó-B", "nó-C", "nó-D", "nó-E"] {
        cluster.add_node(id.into());
    }

    let nodes: Vec<NodeId> = cluster.nodes.keys().cloned().collect();
    let mut storage = Storage::new();
    let mut engine = ConsensusEngine::new(nodes.len());
    let mut graph = Graph::new();

    // 🎯 Populate graph with base vertices
    graph.add_vertex(Vertex::new("A", "Person"));
    graph.add_vertex(Vertex::new("B", "Place"));

    // 🧾 Build proposal content as JSON (simulating an edge insertion)
    let content = serde_json::json!({
        "action": "add_edge",
        "from": "A",
        "to": "B",
        "label": "visits"
    })
    .to_string();

    // 📤 Submit the proposal to the consensus engine
    println!("📤 Submitting proposal...");
    let proposal = engine.submit_proposal("nó-A".into(), content.clone());
    storage.log_proposal(proposal.clone());

    // 🗳️ Each node independently votes on the proposal (simulated logic)
    println!("🕒 Simulating voting...");
    for node in cluster.nodes.values() {
        let vote = node_decide_vote(&node, &proposal);
        storage.log_vote(&proposal.id, node.id.clone(), vote.clone());
        engine.receive_vote(&proposal.id, node.id.clone(), vote);
    }

    // 📊 Evaluate all votes and determine result
    println!("\n📊 Evaluating proposal...");
    let result = engine.evaluate_proposals().pop().unwrap();
    storage.log_result(&proposal.id, result.clone());

    // 🔧 If approved, apply the graph mutation described in the proposal
    if result.approved {
        if let Ok(data) = serde_json::from_str::<Value>(&proposal.content) {
            if data["action"] == "add_edge" {
                let from = data["from"].as_str().unwrap_or("");
                let to = data["to"].as_str().unwrap_or("");
                let label = data["label"].as_str().unwrap_or("related_to");

                graph.add_edge(Edge::new(from, to, label));
                println!(
                    "✅ Edge added to graph: [{}] --{}--> [{}]",
                    from, label, to
                );
            }
        }
    } else {
        println!("❌ Proposal rejected — graph remains unchanged.");
    }

    // 📌 Output the final graph and summary
    println!("\n📌 Final state of the graph:");
    graph.print_graph();

    storage.print_summary();
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
