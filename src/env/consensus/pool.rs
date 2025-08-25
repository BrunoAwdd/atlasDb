use crate::env::proposal::Proposal;

/// Estrutura simples para armazenar e gerenciar propostas em memória.
#[derive(Debug, Default, Clone)]
pub struct ProposalPool {
    proposals: Vec<Proposal>,
}

impl ProposalPool {
    /// Cria um novo pool vazio.
    pub fn new() -> Self {
        Self {
            proposals: Vec::new(),
        }
    }

    /// Adiciona uma nova proposta ao pool.
    pub fn add(&mut self, proposal: Proposal) {
        self.proposals.push(proposal);
    }

    /// Retorna uma referência imutável para todas as propostas.
    pub fn all(&self) -> &[Proposal] {
        &self.proposals
    }


    /// Limpa todas as propostas do pool.
    pub fn clear(&mut self) {
        self.proposals.clear();
    }

    /// Encontra uma proposta por ID (caso necessário).
    pub fn find_by_id(&self, id: &str) -> Option<&Proposal> {
        self.proposals.iter().find(|p| p.id == id)
    }
}
