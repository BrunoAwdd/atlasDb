use std::collections::HashSet;
use tracing::info;

use atlas_common::{
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
        let bft_quorum = 2 * f + 1;
        let quorum_count = std::cmp::max(bft_quorum, self.policy.min_voters);

        info!(
            "🗳️ Avaliando consenso BFT (nós: {}, f: {}, quorum: {})",
            n, f, quorum_count
        );

        let mut results = Vec::new();

        // Iterate over all proposals and phases
        use atlas_common::env::consensus::types::ConsensusPhase;
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

    /// Avalia consenso ponderado por Stake (Weighted Quorum).
    /// Requer acesso ao Ledger para consultar saldos.
    pub async fn evaluate_weighted(
        &self,
        registry: &VoteRegistry,
        active_nodes: &HashSet<NodeId>,
        ledger: &atlas_ledger::Ledger,
    ) -> Vec<ConsensusResult> {
        // 1. Calculate Total Active Stake and Map NodeId -> Stake
        let mut total_active_stake: u64 = 0;
        let mut node_stakes: std::collections::HashMap<NodeId, u64> = std::collections::HashMap::new();

        use atlas_p2p::utils::node_id_to_address;

        for node_id in active_nodes {
            let stake = if let Some(addr) = node_id_to_address(&node_id.0) {
                 ledger.get_validator_total_power(&addr).await.unwrap_or(0)
            } else {
                0
            };
            
            if stake > 0 {
                node_stakes.insert(node_id.clone(), stake);
                total_active_stake += stake;
            }
        }

        // Safety Fallback: If 0 stake found (Genesis not applied?), revert to Count-based Quorum?
        // Or fail safe (no consensus). Fail safe is better for security.
        if total_active_stake == 0 {
            info!("⚠️ Total Active Stake is 0. Cannot reach weighted consensus.");
            return Vec::new(); // Stalemate
        }

        // 2. Calculate Quorum Threshold: > 2/3 of Total Active Stake
        // Q = floor(Total * 2 / 3) + 1
        let quorum_stake = (total_active_stake * 2) / 3 + 1;

        info!(
            "🗳️ Avaliando consenso PONDERADO (nós: {}, Total Stake: {}, Quorum Stake: {})",
            active_nodes.len(), total_active_stake, quorum_stake
        );

        let mut results = Vec::new();
        use atlas_common::env::consensus::types::ConsensusPhase;
        let phases = [ConsensusPhase::Prepare, ConsensusPhase::PreCommit, ConsensusPhase::Commit];

        for (proposal_id, _) in registry.all() {
            for phase in &phases {
                // Sum 'Yes' votes stake
                let mut yes_stake: u64 = 0;
                
                if let Some(votes) = registry.get_votes(proposal_id, phase) {
                    for (voter, vote) in votes {
                        if matches!(vote, Vote::Yes) {
                            yes_stake += node_stakes.get(voter).unwrap_or(&0);
                        }
                    }
                }

                let approved = yes_stake >= quorum_stake;

                if approved {
                    results.push(ConsensusResult {
                        approved,
                        votes_received: 0, // Legacy field, maybe put yes_stake? or just 0
                        proposal_id: proposal_id.clone(),
                        phase: phase.clone(),
                    });

                    info!(
                        "🗳️ Proposta [{}] Fase {:?}: {}/{} Stake — ✅ APROVADA (Weighted)",
                        proposal_id, phase, yes_stake, quorum_stake
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
    use crate::consensus::registry::VoteRegistry;
    use atlas_common::{
    env::consensus::types::{Vote, ConsensusResult, ConsensusPhase},
    utils::NodeId
};

use atlas_p2p::PeerManager;
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
        registry.register_vote(&proposal_id, 0, ConsensusPhase::Prepare, NodeId("node0".into()), Vote::Yes).unwrap();
        let results = evaluator.evaluate(&registry, &active_nodes);
        assert!(results.is_empty());

        // Vote 2: Not enough
        registry.register_vote(&proposal_id, 0, ConsensusPhase::Prepare, NodeId("node1".into()), Vote::Yes).unwrap();
        let results = evaluator.evaluate(&registry, &active_nodes);
        assert!(results.is_empty());

        // Vote 3: Quorum reached!
        registry.register_vote(&proposal_id, 0, ConsensusPhase::Prepare, NodeId("node2".into()), Vote::Yes).unwrap();
        let results = evaluator.evaluate(&registry, &active_nodes);
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].phase, ConsensusPhase::Prepare);
        assert!(results[0].approved);
    }
}
