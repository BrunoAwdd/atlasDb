//! storage.rs
//!
//! Simulates a simple persistence layer for tracking proposals,
//! votes, and final consensus results in a distributed system.
//!
//! This module is designed for testing, logging, and potential future
//! integration with real persistence mechanisms (e.g., database, disk, etc.).
//! 
pub mod audit;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use audit::AuditData;

use super::{
    consensus::{ConsensusResult, Vote},
    proposal::Proposal,
    NodeId,
};

/// In-memory simulation of a distributed storage ledger.
///
/// Used to persist proposals, vote traces, and final consensus outcomes.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Storage {
    /// All proposals submitted to the system.
    pub proposals: Vec<Proposal>,

    /// Map of proposal ID → (node ID → vote).
    pub votes: HashMap<String, HashMap<NodeId, Vote>>,

    /// Map of proposal ID → final consensus result.
    pub results: HashMap<String, ConsensusResult>,
}

impl Storage {
    /// Constructs an empty storage instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Logs a newly submitted proposal.
    ///
    /// This allows the system to retain proposal metadata for future auditing.
    pub fn log_proposal(&mut self, proposal: Proposal) {
        println!("📝 Storing proposal [{}]", proposal.id);
        self.proposals.push(proposal);
    }

    /// Logs a vote submitted by a node for a given proposal.
    ///
    /// Votes are stored per proposal and are associated with the node that cast them.
    pub fn log_vote(&mut self, proposal_id: &str, node: NodeId, vote: Vote) {
        println!("🧾 Logging vote from [{}] on [{}]", node, proposal_id);
        self.votes
            .entry(proposal_id.to_string())
            .or_default()
            .insert(node, vote);
    }

    /// Logs the final consensus result for a given proposal.
    ///
    /// Typically called after quorum evaluation is complete.
    pub fn log_result(&mut self, proposal_id: &str, result: ConsensusResult) {
        println!(
            "📌 Storing result for proposal [{}]: {}",
            proposal_id,
            if result.approved { "✅ APPROVED" } else { "❌ REJECTED" }
        );
        self.results.insert(proposal_id.to_string(), result);
    }

    /// Prints a summary report of all proposals and their outcomes.
    ///
    /// This is primarily for debugging or auditing purposes.
    pub fn print_summary(&self) {
        println!("\n📋 FINAL SUMMARY");

        for prop in &self.proposals {
            let result = self.results.get(&prop.id);
            println!(
                "- [{}] \"{}\" → {}",
                prop.id,
                prop.content,
                match result {
                    Some(r) if r.approved => "✅ APPROVED",
                    Some(_) => "❌ REJECTED",
                    None => "⏳ NO RESULT",
                }
            );
        }
    }

    pub fn to_audit(&self) -> AuditData {
        AuditData {
            proposals: self.proposals.clone(),
            votes: self.votes.clone(),
            results: self.results.clone(),
        }
    }

    pub fn apply_audit(&mut self, data: AuditData) {
        self.proposals = data.proposals;
        self.votes = data.votes;
        self.results = data.results;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::NodeId;

    fn node(id: &str) -> NodeId {
        NodeId(id.to_string())
    }

    fn sample_proposal(id: &str, proposer: &str, content: &str) -> Proposal {
        Proposal {
            id: id.to_string(),
            proposer: node(proposer),
            content: content.to_string(),
            parent: None,
            signature: vec![],
            public_key: vec![],
        }
    }

    fn sample_result(approved: bool, votes: usize, proposal_id: &str) -> ConsensusResult {
        ConsensusResult {
            approved,
            votes_received: votes,
            proposal_id: proposal_id.to_string(),
        }
    }

    #[test]
    fn test_log_proposal_stores_correctly() {
        let mut store = Storage::new();
        let proposal = sample_proposal("p1", "n1", "create edge");

        store.log_proposal(proposal.clone());

        assert_eq!(store.proposals.len(), 1);
        assert_eq!(store.proposals[0].id, "p1");
        assert_eq!(store.proposals[0].content, "create edge");
        assert_eq!(store.proposals[0].proposer, node("n1"));
    }

    #[test]
    fn test_log_vote_adds_vote_entry() {
        let mut store = Storage::new();
        store.log_vote("p1", node("n1"), Vote::Yes);
        store.log_vote("p1", node("n2"), Vote::No);

        let votes = store.votes.get("p1").unwrap();
        assert_eq!(votes.len(), 2);
        assert_eq!(votes.get(&node("n1")), Some(&Vote::Yes));
        assert_eq!(votes.get(&node("n2")), Some(&Vote::No));
    }

    #[test]
    fn test_log_result_registers_outcome() {
        let mut store = Storage::new();
        let result = sample_result(true, 3, "p42");

        store.log_result("p42", result.clone());

        assert!(store.results.contains_key("p42"));
        assert_eq!(store.results["p42"].approved, true);
        assert_eq!(store.results["p42"].votes_received, 3);
    }

    #[test]
    fn test_vote_overwrite_behavior() {
        let mut store = Storage::new();
        store.log_vote("p1", node("n1"), Vote::No);
        store.log_vote("p1", node("n1"), Vote::Yes); // overwrite

        let votes = store.votes.get("p1").unwrap();
        assert_eq!(votes.len(), 1); // still 1 voter
        assert_eq!(votes.get(&node("n1")), Some(&Vote::Yes));
    }

    #[test]
    fn test_print_summary_handles_all_states() {
        let mut store = Storage::new();

        let p1 = sample_proposal("p1", "n1", "A → B");
        let p2 = sample_proposal("p2", "n2", "B → C");
        let p3 = sample_proposal("p3", "n3", "X → Y");

        store.log_proposal(p1.clone());
        store.log_proposal(p2.clone());
        store.log_proposal(p3.clone());

        store.log_result("p1", sample_result(true, 3, "p1"));
        store.log_result("p2", sample_result(false, 1, "p2"));
        // p3 sem resultado

        // Isso imprime no stdout, mas não afeta assertivas aqui.
        store.print_summary();

        assert_eq!(store.results["p1"].approved, true);
        assert_eq!(store.results["p2"].approved, false);
        assert!(!store.results.contains_key("p3")); // sem resultado ainda
    }
}
