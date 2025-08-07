use std::collections::HashSet;

use crate::utils::NodeId;

use super::{
    registry::VoteRegistry,
    types::{ConsensusResult, Vote},
};

/// Componente responsável por avaliar consenso com base em votos e quorum.
#[derive(Debug, Clone)]
pub struct ConsensusEvaluator {
    pub quorum_ratio: f64, // Ex: 0.5 para maioria simples, 0.66 para 2/3
}

impl ConsensusEvaluator {
    pub fn new(quorum_ratio: f64) -> Self {
        Self { quorum_ratio }
    }

    /// Avalia os resultados de consenso para todas as propostas registradas.
    pub fn evaluate(
        &self,
        registry: &VoteRegistry,
        active_nodes: &HashSet<NodeId>,
    ) -> Vec<ConsensusResult> {
        let quorum_count =
            ((active_nodes.len() as f64) * self.quorum_ratio).ceil() as usize;

        println!(
            "🗳️ Avaliando consenso (nós ativos: {}, quorum: {:.2} → {})",
            active_nodes.len(),
            self.quorum_ratio,
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

            println!(
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
