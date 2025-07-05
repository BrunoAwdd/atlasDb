use std::fs;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::env::consensus::{Proposal, Vote, ConsensusResult};
use crate::utils::NodeId;

/// Structure that represents the full audit data of a consensus session.
///
/// It includes:
/// - All submitted proposals.
/// - Votes cast by each node.
/// - Final consensus results for each proposal.
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AuditData {
    /// List of all proposals submitted to the system.
    pub proposals: Vec<Proposal>,

    /// Mapping of proposal ID to a map of node IDs and their corresponding votes.
    pub votes: HashMap<String, HashMap<NodeId, Vote>>,

    /// Mapping of proposal ID to the final consensus result.
    pub results: HashMap<String, ConsensusResult>,
}

/// Saves audit data to a JSON file in pretty format.
///
/// # Parameters
/// - `path`: The path to the file where the data will be written.
/// - `data`: Reference to the `AuditData` to be saved.
///
/// # Returns
/// `Ok(())` on success, or an I/O error if the operation fails.
pub fn save_audit(path: &str, data: &AuditData) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    fs::write(path, json)?;
    Ok(())
}

/// Loads audit data from a JSON file.
///
/// # Parameters
/// - `path`: The path to the file to read.
///
/// # Returns
/// An `AuditData` instance parsed from the file, or an I/O error if reading or parsing fails.
pub fn load_audit(path: &str) -> std::io::Result<AuditData> {
    let json = fs::read_to_string(path)?;
    let data: AuditData = serde_json::from_str(&json)?;
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use crate::env::consensus::{Proposal, Vote, ConsensusResult};

    #[test]
    fn test_save_and_load_audit_data() {
        let mut proposals = Vec::new();
        let mut votes = HashMap::new();
        let mut results = HashMap::new();

        // Simulates a proposal
        let proposal = Proposal {
            id: "prop-123".to_string(),
            proposer: NodeId("node-A".into()),
            content: "Connect A to B".to_string(),
            parent: None,
        };
        proposals.push(proposal.clone());

        // Simulates a Vote
        let mut vote_map = HashMap::new();
        vote_map.insert(NodeId("node-A".to_string()), Vote::Yes);
        votes.insert(proposal.id.clone(), vote_map);

        // Simulates a consensus result
        let result = ConsensusResult {
            proposal_id: proposal.id.clone(),
            approved: true,
            votes_received: 1,
        };
        results.insert(proposal.id.clone(), result);

        let data = AuditData {
            proposals,
            votes,
            results,
        };

        // Save to a temporary file
        let file = NamedTempFile::new().expect("Failed to create temp file");
        let path = file.path().to_str().unwrap();

        save_audit(path, &data).expect("Failed to save audit");

        // Read the file and compare
        let loaded = load_audit(path).expect("Failed to load audit");

        assert_eq!(loaded.proposals.len(), 1);
        assert_eq!(loaded.votes.len(), 1);
        assert_eq!(loaded.results.len(), 1);

        let loaded_proposal = &loaded.proposals[0];
        assert_eq!(loaded_proposal.id, "prop-123");
        assert_eq!(loaded.votes["prop-123"][&NodeId("node-A".to_string())], Vote::Yes);
        assert_eq!(loaded.results["prop-123"].approved, true);
    }
}
