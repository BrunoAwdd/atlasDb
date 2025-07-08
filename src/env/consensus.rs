//! consensus.rs
//!
//! Asynchronous consensus simulation engine with probabilistic voting and quorum evaluation.
//!
//! This module simulates the core logic of a distributed consensus protocol,
//! where nodes vote independently on proposals and quorum is used to determine acceptance.
//!
//! The engine is deliberately asynchronous, failure-tolerant, and latency-aware,
//! serving as a conceptual foundation rather than a production-grade implementation.

use std::{collections::{HashMap, HashSet}, sync::{Arc, RwLock}};
use serde::{Serialize, Deserialize};
use super::{
    super::{peer_manager::PeerManager, utils::NodeId},
    proposal::Proposal
};

/// Represents a binary vote from a node regarding a proposal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Vote {
    Yes,
    No,
}
/// The result of consensus evaluation for a single proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    /// Whether the proposal reached quorum and was approved.
    pub approved: bool,

    /// The number of affirmative (Yes) votes received.
    pub votes_received: usize,

    /// The proposal ID this result corresponds to.
    pub proposal_id: String,
}

/// The core asynchronous consensus engine.
///
/// Manages the lifecycle of proposals, tracks votes,
/// and determines approval based on a quorum threshold.
#[derive(Debug, Clone)]
pub struct ConsensusEngine {
    pub peer_manager: Arc<RwLock<PeerManager>>,
    pub proposals: Vec<Proposal>,
    pub votes: HashMap<String, HashMap<NodeId, Vote>>,
    pub quorum_ratio: f64,
}

impl ConsensusEngine {
    /// Initializes a new consensus engine with a given cluster size.
    ///
    /// Quorum is calculated as majority: (n / 2) + 1
    pub fn new(peer_manager: Arc<RwLock<PeerManager>>, quorum_ratio: f64) -> Self {
        Self {
            proposals: Vec::new(),
            votes: HashMap::new(),
            quorum_ratio,
            peer_manager,
        }
    }

    /// Submits a new proposal to the consensus engine.
    ///
    /// The proposal is tracked and awaits votes from peer nodes.
    pub fn submit_proposal(&mut self, proposer: NodeId, content: String, parent: Option<String>) -> Proposal {
        println!("📝 [{}] submitted proposal: {}", proposer, content);
        let id = format!("prop-{}", self.proposals.len() + 1);
        let proposal = Proposal {
            id: id.clone(),
            proposer,
            content,
            parent
        };
        self.proposals.push(proposal.clone());
        self.votes.insert(id, HashMap::new());
        proposal
    }

    /// Registers a vote from a peer node on a specific proposal.
    ///
    /// This method simulates asynchronous, out-of-order voting from the network.
    pub fn receive_vote(&mut self, proposal_id: &str, from_node: NodeId, vote: Vote) {
        if !self.get_active_nodes().contains(&from_node) {
            println!("⚠️ Ignored vote from unknown or inactive node: [{}]", from_node);
            return;
        }

        if let Some(voters) = self.votes.get_mut(proposal_id) {
            voters.insert(from_node.clone(), vote.clone());

            println!(
                "📥 [{}] voted {:?} on proposal [{}]",
                from_node, vote, proposal_id
            );
        }
    }

    /// Evaluates all tracked proposals and computes consensus results.
    ///
    /// Proposals are considered approved if they receive quorum (≥ majority) of `Yes` votes.
    pub fn evaluate_proposals(&self) -> Vec<ConsensusResult> {
        let quorum_count = (self.get_active_nodes().len() as f64 * self.quorum_ratio).ceil() as usize;
        let mut results = Vec::new();

        for (id, voters) in &self.votes {
            let yes_votes = voters.values().filter(|v| matches!(v, Vote::Yes)).count();
            let approved = yes_votes >= quorum_count;

            results.push(ConsensusResult {
                approved,
                votes_received: yes_votes,
                proposal_id: id.clone(),
            });

            println!(
                "🗳️ Proposal [{}] received {}/{} YES votes — {}",
                id,
                yes_votes,
                quorum_count,
                if approved { "APPROVED ✅" } else { "REJECTED ❌" }
            );
        }

        results
    }

    pub fn update_active_nodes(&mut self) {
        println!("Not implemented");
    }

    fn get_active_nodes(&self) -> HashSet<NodeId> {
        self.peer_manager.read().expect("Failed to acquire read lock").get_active_peers()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::NodeId;

    fn create_engine_with_nodes(n: usize) -> (ConsensusEngine, Vec<NodeId>) {
        let nodes: Vec<NodeId> = (0..n)
            .map(|i| NodeId(format!("node-{}", i)))
            .collect();

        let peer_manager = Arc::new(RwLock::new(PeerManager::new(nodes.len(), 5)));
        let quorum_ratio = 0.66;

        let engine = ConsensusEngine::new(peer_manager, quorum_ratio);
        (engine, nodes)
    }

    #[test]
    fn test_submit_proposal_creates_entry() {
        let (mut engine, nodes) = create_engine_with_nodes(3);
        let proposal = engine.submit_proposal(nodes[0].clone(), "add_edge A-B".into(), None);

        assert_eq!(proposal.id, "prop-1");
        assert_eq!(proposal.content, "add_edge A-B");
        assert!(engine.votes.contains_key(&proposal.id));
        assert_eq!(engine.proposals.len(), 1);
    }

    #[test]
    fn test_receive_vote_registers_vote_correctly() {
        let (mut engine, nodes) = create_engine_with_nodes(3);
        let proposal = engine.submit_proposal(nodes[0].clone(), "connect X-Y".into(), None);

        engine.receive_vote(&proposal.id, nodes[1].clone(), Vote::Yes);

        let voters = engine.votes.get(&proposal.id).unwrap();
        assert_eq!(voters.get(&nodes[1]), Some(&Vote::Yes));
    }

    #[test]
    fn test_quorum_approval_success() {
        let (mut engine, nodes) = create_engine_with_nodes(5); // quorum = 3
        let proposal = engine.submit_proposal(nodes[0].clone(), "edge A-B".into(), None);

        engine.receive_vote(&proposal.id, nodes[1].clone(), Vote::Yes);
        engine.receive_vote(&proposal.id, nodes[2].clone(), Vote::Yes);
        engine.receive_vote(&proposal.id, nodes[3].clone(), Vote::Yes);

        let results = engine.evaluate_proposals();
        assert_eq!(results.len(), 1);
        assert!(results[0].approved);
        assert_eq!(results[0].votes_received, 3);
    }

    #[test]
    fn test_quorum_rejection_due_to_insufficient_votes() {
        let (mut engine, nodes) = create_engine_with_nodes(5); // quorum = 3
        let proposal = engine.submit_proposal(nodes[0].clone(), "edge A-B".into(), None);

        engine.receive_vote(&proposal.id, nodes[1].clone(), Vote::Yes);
        engine.receive_vote(&proposal.id, nodes[2].clone(), Vote::No);

        let results = engine.evaluate_proposals();
        assert!(!results[0].approved);
        assert_eq!(results[0].votes_received, 1);
    }

    #[test]
    fn test_out_of_order_votes_do_not_affect_result() {
        let (mut engine, nodes) = create_engine_with_nodes(3); // quorum = 2
        let proposal = engine.submit_proposal(nodes[2].clone(), "modify X".into(), None);

        // Votes arrive in "wrong" order — this should not matter
        engine.receive_vote(&proposal.id, nodes[1].clone(), Vote::Yes);
        engine.receive_vote(&proposal.id, nodes[0].clone(), Vote::Yes);

        let results = engine.evaluate_proposals();
        assert!(results[0].approved);
    }

    #[test]
    fn test_multiple_proposals_independent_results() {
        let (mut engine, nodes) = create_engine_with_nodes(3); // quorum = 2

        let p1 = engine.submit_proposal(nodes[0].clone(), "edge A-B".into(), None);
        let p2 = engine.submit_proposal(nodes[1].clone(), "edge C-D".into(), None);

        engine.receive_vote(&p1.id, nodes[1].clone(), Vote::Yes);
        engine.receive_vote(&p1.id, nodes[2].clone(), Vote::Yes);

        engine.receive_vote(&p2.id, nodes[2].clone(), Vote::No);
        engine.receive_vote(&p2.id, nodes[0].clone(), Vote::No);

        let results = engine.evaluate_proposals();

        assert_eq!(results.len(), 2);

        let mut approved_ids = vec![];
        let mut rejected_ids = vec![];

        for result in results {
            if result.approved {
                approved_ids.push(result.proposal_id.clone());
            } else {
                rejected_ids.push(result.proposal_id.clone());
            }
        }

        assert!(approved_ids.contains(&p1.id));
        assert!(rejected_ids.contains(&p2.id));
    }
}
