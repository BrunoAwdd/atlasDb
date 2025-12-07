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
    proposal::Proposal,
};

use atlas_sdk::{
    utils::NodeId,
    env::consensus::types::{ConsensusResult, Vote, ConsensusPhase},
};

use crate::ledger::Ledger;
use std::sync::Arc;

/// In-memory simulation of a distributed storage ledger.
///
/// Used to persist proposals, vote traces, and final consensus outcomes.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Storage {
    /// All proposals submitted to the system.
    pub proposals: Vec<Proposal>,

    /// Map of proposal ID → Phase → (node ID → vote).
    pub votes: HashMap<String, HashMap<ConsensusPhase, HashMap<NodeId, Vote>>>,

    /// Map of proposal ID → final consensus result.
    pub results: HashMap<String, ConsensusResult>,

    /// Persistent Ledger (Binlog + RocksDB)
    #[serde(skip)]
    pub ledger: Option<Arc<Ledger>>,
}

impl Storage {
    /// Constructs an empty storage instance.
    pub fn new(data_dir: &str) -> Self {
        // Since this is called from a synchronous context (builder::init), we need to block.
        // We create a temporary runtime for this initialization.
        // To avoid "Cannot start a runtime from within a runtime" panic (if called from async context like setup.rs),
        // we spawn a dedicated thread.
        let data_dir = data_dir.to_string();
        let ledger = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            rt.block_on(async {
                Ledger::new(&data_dir).await.expect("Failed to initialize Ledger")
            })
        }).join().expect("Failed to join thread");

        Self {
            proposals: Vec::new(),
            votes: HashMap::new(),
            results: HashMap::new(),
            ledger: Some(Arc::new(ledger)),
        }
    }

    /// Logs a newly submitted proposal.
    ///
    /// This allows the system to retain proposal metadata for future auditing.
    pub fn log_proposal(&mut self, proposal: Proposal) {
        println!("📝 Storing proposal [{}]", proposal.id);
        self.proposals.push(proposal.clone());
        
        // Persist to Ledger
        if let Some(ledger) = &self.ledger {
            let ledger = ledger.clone();
            let prop = proposal.clone();
            tokio::spawn(async move {
                if let Err(e) = ledger.append_proposal(&prop).await {
                    eprintln!("❌ Failed to append proposal to ledger: {}", e);
                }
            });
        } else {
            eprintln!("⚠️ Ledger not initialized, proposal not persisted to disk!");
        }
    }

    /// Logs a vote submitted by a node for a given proposal in a specific phase.
    ///
    /// Votes are stored per proposal/phase and are associated with the node that cast them.
    pub fn log_vote(&mut self, proposal_id: &str, phase: ConsensusPhase, node: NodeId, vote: Vote) {
        println!("🧾 Logging vote from [{}] on [{}] (Phase: {:?})", node, proposal_id, phase);
        self.votes
            .entry(proposal_id.to_string())
            .or_default()
            .entry(phase)
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

    /// Returns all proposals with height greater than the given height.
    pub async fn get_proposals_after(&self, height: u64) -> Vec<Proposal> {
        if let Some(ledger) = &self.ledger {
            match ledger.get_proposals_after(height).await {
                Ok(proposals) => return proposals,
                Err(e) => eprintln!("❌ Failed to read from ledger: {}", e),
            }
        }
        
        // Fallback to in-memory (mostly for tests or if ledger fails)
        self.proposals.iter()
            .filter(|p| p.height > height)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_sdk::{
        utils::NodeId,
    };

    fn node(id: &str) -> NodeId {
        NodeId(id.to_string())
    }

    fn sample_proposal(id: &str, proposer: &str, content: &str) -> Proposal {
        Proposal {
            id: id.to_string(),
            proposer: node(proposer),
            content: content.to_string(),
            parent: None,
            height: 0,
            signature: [0u8; 64],
            public_key: vec![],
        }
    }

    fn sample_result(approved: bool, votes: usize, proposal_id: &str) -> ConsensusResult {
        ConsensusResult {
            approved,
            votes_received: votes,
            proposal_id: proposal_id.to_string(),
            phase: atlas_sdk::env::consensus::types::ConsensusPhase::Commit,
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
        store.log_vote("p1", ConsensusPhase::Prepare, node("n1"), Vote::Yes);
        store.log_vote("p1", ConsensusPhase::Prepare, node("n2"), Vote::No);

        let phases = store.votes.get("p1").unwrap();
        let votes = phases.get(&ConsensusPhase::Prepare).unwrap();
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
        store.log_vote("p1", ConsensusPhase::Prepare, node("n1"), Vote::No);
        store.log_vote("p1", ConsensusPhase::Prepare, node("n1"), Vote::Yes); // overwrite

        let phases = store.votes.get("p1").unwrap();
        let votes = phases.get(&ConsensusPhase::Prepare).unwrap();
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
