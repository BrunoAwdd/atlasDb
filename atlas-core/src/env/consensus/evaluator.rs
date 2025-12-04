use std::collections::HashSet;
use tracing::info;

use atlas_sdk::{
    utils::NodeId,
    env::consensus::types::{ConsensusResult, Vote},
};

use super::{
    registry::VoteRegistry,
};

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumPolicy {
    pub fraction: f64,
    pub min_voters: usize,
}

impl Default for QuorumPolicy {
    fn default() -> Self {
        Self { fraction: 0.5, min_voters: 1 }
    }
}

/// Componente responsável por avaliar consenso com base em votos e quorum.
#[derive(Debug, Clone)]
pub struct ConsensusEvaluator {
    pub policy: QuorumPolicy,
}

impl ConsensusEvaluator {
    pub fn new(policy: QuorumPolicy) -> Self {
        Self { policy }
    }

    /// Avalia os resultados de consenso para todas as propostas registradas.
    pub fn evaluate(
        &self,
        registry: &VoteRegistry,
        active_nodes: &HashSet<NodeId>,
    ) -> Vec<ConsensusResult> {
        let total_nodes = active_nodes.len();
        let fraction_required = ((total_nodes as f64) * self.policy.fraction).ceil() as usize;
        let quorum_count = std::cmp::max(fraction_required, self.policy.min_voters);

        info!(
            "🗳️ Avaliando consenso (nós ativos: {}, policy: {:.2}/{}, necessário: {})",
            total_nodes,
            self.policy.fraction,
            self.policy.min_voters,
            quorum_count
        );

        let mut results = Vec::new();

        for (proposal_id, votes) in registry.all() {
            let yes_votes = votes.values().filter(|v| matches!(v, Vote::Yes)).count();
            let approved = yes_votes >= quorum_count;

            results.push(ConsensusResult {
                approved,
                votes_received: yes_votes,
                proposal_id: proposal_id.clone(),
            });

            info!(
                "🗳️ Proposta [{}]: {}/{} votos 'Yes' — {}",
                proposal_id,
                yes_votes,
                quorum_count,
                if approved { "✅ APROVADA" } else { "❌ REJEITADA" }
            );
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::consensus::registry::VoteRegistry;
    use atlas_sdk::env::consensus::types::Vote;

    #[test]
    fn test_quorum_policy_fraction() {
        let policy = QuorumPolicy { fraction: 0.5, min_voters: 1 };
        let evaluator = ConsensusEvaluator::new(policy);
        let mut registry = VoteRegistry::new();
        let active_nodes: HashSet<NodeId> = vec![
            NodeId("node1".into()), NodeId("node2".into()), NodeId("node3".into())
        ].into_iter().collect();

        // 3 nodes, 0.5 fraction -> ceil(1.5) = 2 votes needed.

        let proposal_id = "prop1".to_string();
        registry.register_proposal(&proposal_id);
        registry.register_vote(&proposal_id, NodeId("node1".into()), Vote::Yes);
        
        let results = evaluator.evaluate(&registry, &active_nodes);
        assert!(!results[0].approved, "Should fail with 1/3 votes");

        registry.register_vote(&proposal_id, NodeId("node2".into()), Vote::Yes);
        let results = evaluator.evaluate(&registry, &active_nodes);
        assert!(results[0].approved, "Should pass with 2/3 votes");
    }

    #[test]
    fn test_quorum_policy_min_voters() {
        let policy = QuorumPolicy { fraction: 0.1, min_voters: 3 }; // fraction gives 0.4 -> 1, but min is 3
        let evaluator = ConsensusEvaluator::new(policy);
        let mut registry = VoteRegistry::new();
        let active_nodes: HashSet<NodeId> = vec![
            NodeId("node1".into()), NodeId("node2".into()), NodeId("node3".into()), NodeId("node4".into())
        ].into_iter().collect();

        let proposal_id = "prop2".to_string();
        registry.register_proposal(&proposal_id);
        registry.register_vote(&proposal_id, NodeId("node1".into()), Vote::Yes);
        registry.register_vote(&proposal_id, NodeId("node2".into()), Vote::Yes);

        let results = evaluator.evaluate(&registry, &active_nodes);
        assert!(!results[0].approved, "Should fail with 2 votes (min 3)");

        registry.register_vote(&proposal_id, NodeId("node3".into()), Vote::Yes);
        let results = evaluator.evaluate(&registry, &active_nodes);
        assert!(results[0].approved, "Should pass with 3 votes");
    }
}
