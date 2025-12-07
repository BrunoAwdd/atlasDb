use std::collections::HashMap;

use atlas_sdk::{
    utils::NodeId,
    env::consensus::types::{Vote, ConsensusPhase},
};

/// Armazena os votos de cada nó para cada proposta, separados por fase.
#[derive(Debug, Default, Clone)]
pub struct VoteRegistry {
    // ProposalID -> Phase -> NodeID -> Vote
    votes: HashMap<String, HashMap<ConsensusPhase, HashMap<NodeId, Vote>>>,
}

impl VoteRegistry {
    /// Cria um novo registro de votos vazio.
    pub fn new() -> Self {
        Self {
            votes: HashMap::new(),
        }
    }

    /// Inicializa o mapa de votos para uma nova proposta.
    pub fn register_proposal(&mut self, proposal_id: &str) {
        self.votes.entry(proposal_id.to_string()).or_default();
    }

    /// Registra o voto de um nó para uma proposta em uma determinada fase.
    /// Retorna erro se o nó já votou nesta fase (equivocação).
    pub fn register_vote(&mut self, proposal_id: &str, phase: ConsensusPhase, node: NodeId, vote: Vote) -> Result<(), String> {
        let phase_votes = self.votes
            .entry(proposal_id.to_string())
            .or_default()
            .entry(phase.clone())
            .or_default();

        if let Some(existing_vote) = phase_votes.get(&node) {
            if *existing_vote != vote {
                return Err(format!("EQUIVOCATION: Node {} voted {:?} then {:?} in phase {:?}", node, existing_vote, vote, phase));
            }
            // Idempotency: same vote is fine
            return Ok(());
        }

        phase_votes.insert(node, vote);
        Ok(())
    }

    /// Retorna a quantidade de votos "Yes" para uma proposta em uma fase específica.
    pub fn count_yes(&self, proposal_id: &str, phase: &ConsensusPhase) -> usize {
        self.votes
            .get(proposal_id)
            .and_then(|phases| phases.get(phase))
            .map(|m| m.values().filter(|v| matches!(v, Vote::Yes)).count())
            .unwrap_or(0)
    }

    /// Retorna todos os votos de uma proposta em uma fase (se existirem).
    pub fn get_votes(&self, proposal_id: &str, phase: &ConsensusPhase) -> Option<&HashMap<NodeId, Vote>> {
        self.votes.get(proposal_id).and_then(|p| p.get(phase))
    }

    /// Retorna todos os registros de votos (estrutura completa).
    pub fn all(&self) -> &HashMap<String, HashMap<ConsensusPhase, HashMap<NodeId, Vote>>> {
        &self.votes
    }

    /// Substitui os votos manualmente (para carregar estado externo, se necessário).
    pub fn replace(&mut self, new_votes: HashMap<String, HashMap<ConsensusPhase, HashMap<NodeId, Vote>>>) {
        self.votes = new_votes;
    }
}