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
        let n = active_nodes.len();
        // BFT Quorum: f = (n-1)/3, quorum = 2f + 1
        let f = (n.saturating_sub(1)) / 3;
        let quorum_count = 2 * f + 1;

        info!(
            "🗳️ Avaliando consenso BFT (nós: {}, f: {}, quorum: {})",
            n, f, quorum_count
        );

        let mut results = Vec::new();

        // Iterate over all proposals and phases
        use atlas_sdk::env::consensus::types::ConsensusPhase;
        let phases = [ConsensusPhase::Prepare, ConsensusPhase::PreCommit, ConsensusPhase::Commit];

        for (proposal_id, _) in registry.all() {
            for phase in &phases {
                let yes_votes = registry.count_yes(proposal_id, phase);
                let approved = yes_votes >= quorum_count;

                if approved {
                    results.push(ConsensusResult {
                        approved,
                        votes_received: yes_votes,
                        proposal_id: proposal_id.clone(),
                        phase: phase.clone(),
                    });

                    info!(
                        "🗳️ Proposta [{}] Fase {:?}: {}/{} votos 'Yes' — ✅ APROVADA",
                        proposal_id, phase, yes_votes, quorum_count
                    );
                }
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::consensus::registry::VoteRegistry;
    use atlas_sdk::env::consensus::types::{Vote, ConsensusPhase};

    #[test]
    fn test_bft_quorum_calculation() {
        let policy = QuorumPolicy::default();
        let evaluator = ConsensusEvaluator::new(policy);
        let mut registry = VoteRegistry::new();
        
        // 4 nodes -> f=1 -> quorum=3
        let active_nodes: HashSet<NodeId> = (0..4).map(|i| NodeId(format!("node{}", i))).collect();

        let proposal_id = "prop1".to_string();
        registry.register_proposal(&proposal_id);
        
        // Vote 1: Not enough
        registry.register_vote(&proposal_id, ConsensusPhase::Prepare, NodeId("node0".into()), Vote::Yes);
        let results = evaluator.evaluate(&registry, &active_nodes);
        assert!(results.is_empty());

        // Vote 2: Not enough
        registry.register_vote(&proposal_id, ConsensusPhase::Prepare, NodeId("node1".into()), Vote::Yes);
        let results = evaluator.evaluate(&registry, &active_nodes);
        assert!(results.is_empty());

        // Vote 3: Quorum reached!
        registry.register_vote(&proposal_id, ConsensusPhase::Prepare, NodeId("node2".into()), Vote::Yes);
        let results = evaluator.evaluate(&registry, &active_nodes);
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].phase, ConsensusPhase::Prepare);
        assert!(results[0].approved);
    }
}
