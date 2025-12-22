use std::collections::HashMap;

use atlas_common::{
    utils::NodeId,
    env::consensus::types::{Vote, ConsensusPhase},
    env::consensus::evidence::EquivocationEvidence,
};

/// Armazena os votos de cada nó para cada proposta, separados por fase.
#[derive(Debug, Default, Clone)]
pub struct VoteRegistry {
    // ProposalID -> Phase -> NodeID -> Vote
    votes: HashMap<String, HashMap<ConsensusPhase, HashMap<NodeId, Vote>>>,
    
    // View -> Phase -> NodeID -> ProposalID (For detecting different proposals in same view)
    votes_by_view: HashMap<u64, HashMap<ConsensusPhase, HashMap<NodeId, String>>>,
}

impl VoteRegistry {
    /// Cria um novo registro de votos vazio.
    pub fn new() -> Self {
        Self {
            votes: HashMap::new(),
            votes_by_view: HashMap::new(),
        }
    }

    /// Inicializa o mapa de votos para uma nova proposta.
    pub fn register_proposal(&mut self, proposal_id: &str) {
        self.votes.entry(proposal_id.to_string()).or_default();
    }

    /// Registra o voto de um nó para uma proposta em uma determinada fase e view.
    /// Retorna Evidence se detectar equivovação.
    pub fn register_vote(&mut self, proposal_id: &str, view: u64, phase: ConsensusPhase, node: NodeId, vote: Vote) -> Result<Option<EquivocationEvidence>, String> {
        // 1. Check Equivocation (Double Voting on different proposals)
        let phase_view_votes = self.votes_by_view
            .entry(view)
            .or_default()
            .entry(phase.clone())
            .or_default();

        if let Some(existing_proposal) = phase_view_votes.get(&node) {
            if existing_proposal != proposal_id {
                let evidence = EquivocationEvidence {
                    offender: node.clone(),
                    view,
                    phase_step: format!("{:?}", phase),
                    vote_a: Vote::Yes, // We need to fetch the actual other vote, but for now assuming Yes/Yes conflict
                    vote_b: vote.clone(),
                    proposal_a: existing_proposal.clone(),
                    proposal_b: proposal_id.to_string(),
                };
                return Ok(Some(evidence));
            }
        } else {
            phase_view_votes.insert(node.clone(), proposal_id.to_string());
        }

        // 2. Check Conflicting Vote (Same Proposal, Different Vote Value)
        let phase_votes = self.votes
            .entry(proposal_id.to_string())
            .or_default()
            .entry(phase.clone())
            .or_default();

        if let Some(existing_vote) = phase_votes.get(&node) {
            if *existing_vote != vote {
                let evidence = EquivocationEvidence {
                    offender: node.clone(),
                    view,
                    phase_step: format!("{:?}", phase),
                    vote_a: existing_vote.clone(),
                    vote_b: vote.clone(),
                    proposal_a: proposal_id.to_string(),
                    proposal_b: proposal_id.to_string(),
                };
                return Ok(Some(evidence));
            }
            // Idempotency: same vote is fine
            return Ok(None);
        }

        phase_votes.insert(node, vote);
        Ok(None)
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

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_common::env::consensus::types::{Vote, ConsensusPhase};
    use atlas_common::utils::NodeId;

    #[test]
    fn test_valid_votes() {
        let mut registry = VoteRegistry::new();
        let node = NodeId("node1".into());
        
        let res = registry.register_vote("prop1", 1, ConsensusPhase::Prepare, node.clone(), Vote::Yes);
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());

        // Idempotent (same vote)
        let res = registry.register_vote("prop1", 1, ConsensusPhase::Prepare, node.clone(), Vote::Yes);
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
    }

    #[test]
    fn test_equivocation_diff_value() {
        let mut registry = VoteRegistry::new();
        let node = NodeId("node1".into());
        
        registry.register_vote("prop1", 1, ConsensusPhase::Prepare, node.clone(), Vote::Yes).unwrap();
        
        // Same proposal, same view, SAME phase, DIFFERENT value -> Equivocation
        let res = registry.register_vote("prop1", 1, ConsensusPhase::Prepare, node.clone(), Vote::No);
        assert!(res.is_ok()); // It returns Ok(Some(evidence))
        let evidence = res.unwrap();
        assert!(evidence.is_some());
        assert_eq!(evidence.unwrap().offender, node);
    }

    #[test]
    fn test_equivocation_diff_proposal() {
        let mut registry = VoteRegistry::new();
        let node = NodeId("node1".into());
        
        registry.register_vote("prop1", 1, ConsensusPhase::Prepare, node.clone(), Vote::Yes).unwrap();
        
        // DIFFERENT proposal, SAME view, SAME phase -> Equivocation
        let res = registry.register_vote("prop2", 1, ConsensusPhase::Prepare, node.clone(), Vote::Yes);
        assert!(res.is_ok());
        let evidence = res.unwrap();
        assert!(evidence.is_some());
        
        let ev = evidence.unwrap();
        assert_eq!(ev.offender, node);
        assert_eq!(ev.proposal_a, "prop1");
        assert_eq!(ev.proposal_b, "prop2");
    }
    
    #[test]
    fn test_diff_view_ok() {
        let mut registry = VoteRegistry::new();
        let node = NodeId("node1".into());
        
        registry.register_vote("prop1", 1, ConsensusPhase::Prepare, node.clone(), Vote::Yes).unwrap();
        
        // Different View -> OK
        let res = registry.register_vote("prop2", 2, ConsensusPhase::Prepare, node.clone(), Vote::Yes);
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
    }
}