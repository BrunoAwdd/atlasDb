use std::collections::HashMap;

use atlas_common::env::proposal::Proposal;

/// Estrutura simples para armazenar e gerenciar propostas em memória.
#[derive(Debug, Default, Clone)]
pub struct ProposalPool {
    proposals: HashMap<String, Proposal>,
}

impl ProposalPool {
    /// Create a new empty proposal pool.
    pub fn new() -> Self {
        Self {
            proposals: HashMap::new(),
        }
    }

    /// Add new proposal to the pool.
    pub fn add(&mut self, proposal: Proposal) {
        if self.proposals.insert(proposal.clone().id, proposal).is_some() {
            eprintln!("⚠️ Proposal com id já existe no pool");
        }
    }

    /// Remove proposal from the pool.
    pub fn remove(&mut self, id: &str) {
        self.proposals.remove(id);
    }

    /// Get all proposals in the pool.
    pub fn all(&self) -> &HashMap<std::string::String, Proposal> {
        &self.proposals
    }


    /// Limpa todas as propostas do pool.
    pub fn clear(&mut self) {
        self.proposals.clear();
    }

    /// Find propouse by id.
    pub fn find_by_id(&self, id: &str) -> Option<&Proposal> {
        self.proposals.get(id)
    }
}
