use std::collections::HashMap;

use crate::utils::NodeId;
use super::types::Vote;

/// Armazena os votos de cada nó para cada proposta.
#[derive(Debug, Default, Clone)]
pub struct VoteRegistry {
    votes: HashMap<String, HashMap<NodeId, Vote>>,
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

    /// Registra o voto de um nó para uma proposta.
    pub fn register_vote(&mut self, proposal_id: &str, node: NodeId, vote: Vote) {
        self.votes
            .entry(proposal_id.to_string())
            .or_default()
            .insert(node, vote);
    }

    /// Retorna a quantidade de votos "Yes" para uma proposta.
    pub fn count_yes(&self, proposal_id: &str) -> usize {
        self.votes
            .get(proposal_id)
            .map(|m| m.values().filter(|v| matches!(v, Vote::Yes)).count())
            .unwrap_or(0)
    }

    /// Retorna todos os votos de uma proposta (se existirem).
    pub fn get_votes(&self, proposal_id: &str) -> Option<&HashMap<NodeId, Vote>> {
        self.votes.get(proposal_id)
    }

    /// Retorna todos os registros de votos.
    pub fn all(&self) -> &HashMap<String, HashMap<NodeId, Vote>> {
        &self.votes
    }

    /// Substitui os votos manualmente (para carregar estado externo, se necessário).
    pub fn replace(&mut self, new_votes: HashMap<String, HashMap<NodeId, Vote>>) {
        self.votes = new_votes;
    }
}