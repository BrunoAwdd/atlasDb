use std::collections::HashMap;

use atlas_common::{
    utils::NodeId,
    env::consensus::types::{Vote, ConsensusPhase},
    env::consensus::evidence::EquivocationEvidence,
    env::vote_data::VoteData,
};

/// Armazena os votos de cada nó para cada proposta, separados por fase.
#[derive(Debug, Default, Clone)]
pub struct VoteRegistry {
    // ProposalID -> Phase -> NodeID -> VoteData
    votes: HashMap<String, HashMap<ConsensusPhase, HashMap<NodeId, VoteData>>>,
    
    // (Height, View) -> Phase -> NodeID -> VoteData 
    // Key is (Height, View)
    votes_by_height_view: HashMap<(u64, u64), HashMap<ConsensusPhase, HashMap<NodeId, VoteData>>>,
}

impl VoteRegistry {
    /// Cria um novo registro de votos vazio.
    pub fn new() -> Self {
        Self {
            votes: HashMap::new(),
            votes_by_height_view: HashMap::new(),
        }
    }

    /// Inicializa o mapa de votos para uma nova proposta.
    pub fn register_proposal(&mut self, proposal_id: &str) {
        self.votes.entry(proposal_id.to_string()).or_default();
    }

    /// Registra o voto de um nó para uma proposta em uma determinada fase e view.
    /// Retorna Evidence se detectar equivovação.
    pub fn register_vote(&mut self, vote_data: VoteData) -> Result<Option<EquivocationEvidence>, String> {
        let node = vote_data.voter.clone();
        let view = vote_data.view;
        let height = vote_data.height; // New
        let phase = vote_data.phase.clone();
        let proposal_id = vote_data.proposal_id.clone();
        let vote = vote_data.vote.clone();

        // 1. Check Equivocation (Double Voting on different proposals OR same proposal different value)
        
        let phase_view_votes = self.votes_by_height_view
            .entry((height, view))
            .or_default()
            .entry(phase.clone())
            .or_default();

        if let Some(existing_vote) = phase_view_votes.get(&node) {
            // Check for conflict
            let conflict_proposal = existing_vote.proposal_id != proposal_id;
            let conflict_value = existing_vote.vote != vote;

            if conflict_proposal || conflict_value {
                // Return evidence with BOTH signed votes
                let evidence = EquivocationEvidence {
                    vote_a: existing_vote.clone(),
                    vote_b: vote_data.clone(),
                };
                return Ok(Some(evidence));
            }
            // If identical (same proposal, same value), it's just a retry.
        } else {
             phase_view_votes.insert(node.clone(), vote_data.clone());
        }

        // 2. Index by Proposal (for Quorum Counting)
        let phase_votes = self.votes
            .entry(proposal_id.clone())
            .or_default()
            .entry(phase.clone())
            .or_default();
        
        phase_votes.insert(node, vote_data); // Overwrite/Insert

        Ok(None)
    }

    /// Retorna a quantidade de votos "Yes" para uma proposta em uma fase específica.
    pub fn count_yes(&self, proposal_id: &str, phase: &ConsensusPhase) -> usize {
        self.votes
            .get(proposal_id)
            .and_then(|phases| phases.get(phase))
            .map(|m| m.values().filter(|v| matches!(v.vote, Vote::Yes)).count())
            .unwrap_or(0)
    }

    /// Retorna todos os votos de uma proposta em uma fase (se existirem).
    pub fn get_votes(&self, proposal_id: &str, phase: &ConsensusPhase) -> Option<HashMap<NodeId, Vote>> {
        // Adapt return type to match expected simple Map (Node -> Vote Enum) for Evaluator
        // Or refactor Evaluator? Evaluator expects HashMap<NodeId, Vote>.
        // We can map it here.
        self.votes.get(proposal_id)
            .and_then(|p| p.get(phase))
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.vote.clone())).collect())
    }

    /// Retorna todos os registros de votos (estrutura completa).
    // pub fn all(&self) -> &HashMap<String, HashMap<ConsensusPhase, HashMap<NodeId, VoteData>>> {
    //     &self.votes
    // }
    pub fn all(&self) -> Vec<(String, HashMap<ConsensusPhase, HashMap<NodeId, Vote>>)> {
         self.votes.iter().map(|(pid, phases)| {
             let phases_map = phases.iter().map(|(ph, nodes)| {
                 let nodes_map = nodes.iter().map(|(n, vd)| (n.clone(), vd.vote.clone())).collect();
                 (ph.clone(), nodes_map)
             }).collect();
             (pid.clone(), phases_map)
         }).collect()
    }

    // Substitui os votos manualmente (para carregar estado externo, se necessário).
    // Ignoring replace for now as signature changed complexity
    // pub fn replace(&mut self, new_votes: HashMap<String, HashMap<ConsensusPhase, HashMap<NodeId, Vote>>>) {
    //     self.votes = new_votes;
    // }

    pub fn has_voted(&self, height: u64, view: u64, phase: &ConsensusPhase, node: &NodeId) -> bool {
        self.votes_by_height_view
            .get(&(height, view))
            .and_then(|phases| phases.get(phase))
            .map(|nodes| nodes.contains_key(node))
            .unwrap_or(false)
    }

    pub fn get_vote(&self, height: u64, view: u64, phase: &ConsensusPhase, node: &NodeId) -> Option<&VoteData> {
        self.votes_by_height_view
            .get(&(height, view))
            .and_then(|phases| phases.get(phase))
            .and_then(|nodes| nodes.get(node))
    }

    /// Returns the highest view number observed in registered votes (across all heights).
    /// Used for catch-up/sync mostly.
    pub fn get_highest_view(&self) -> Option<u64> {
        self.votes_by_height_view.keys().map(|(_, v)| *v).max()
    }

    /// Clears all votes. Should be called after a successful commit to reset state for the next Height.
    pub fn clear(&mut self) {
        self.votes.clear();
        self.votes_by_height_view.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_common::env::consensus::types::{Vote, ConsensusPhase};
    use atlas_common::env::vote_data::VoteData;
    use atlas_common::utils::NodeId;

    fn mock_vote(prop: &str, view: u64, phase: ConsensusPhase, node: NodeId, vote: Vote) -> VoteData {
        VoteData {
            proposal_id: prop.to_string(),
            vote,
            voter: node,
            phase,
            view,
            signature: [0u8; 64],
            public_key: vec![]
        }
    }

    #[test]
    fn test_valid_votes() {
        let mut registry = VoteRegistry::new();
        let node = NodeId("node1".into());
        
        let v1 = mock_vote("prop1", 1, ConsensusPhase::Prepare, node.clone(), Vote::Yes);
        let res = registry.register_vote(v1);
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());

        // Idempotent (same vote)
        let v2 = mock_vote("prop1", 1, ConsensusPhase::Prepare, node.clone(), Vote::Yes);
        let res = registry.register_vote(v2);
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
    }

    #[test]
    fn test_equivocation_diff_value() {
        let mut registry = VoteRegistry::new();
        let node = NodeId("node1".into());
        
        let v1 = mock_vote("prop1", 1, ConsensusPhase::Prepare, node.clone(), Vote::Yes);
        registry.register_vote(v1).unwrap();
        
        // Same proposal, same view, SAME phase, DIFFERENT value -> Equivocation
        let v2 = mock_vote("prop1", 1, ConsensusPhase::Prepare, node.clone(), Vote::No);
        let res = registry.register_vote(v2);
        assert!(res.is_ok()); // It returns Ok(Some(evidence))
        let evidence = res.unwrap();
        assert!(evidence.is_some());
        assert_eq!(evidence.unwrap().vote_a.voter, node);
    }

    #[test]
    fn test_equivocation_diff_proposal() {
        let mut registry = VoteRegistry::new();
        let node = NodeId("node1".into());
        
        let v1 = mock_vote("prop1", 1, ConsensusPhase::Prepare, node.clone(), Vote::Yes);
        registry.register_vote(v1).unwrap();
        
        // DIFFERENT proposal, SAME view, SAME phase -> Equivocation
        let v2 = mock_vote("prop2", 1, ConsensusPhase::Prepare, node.clone(), Vote::Yes);
        let res = registry.register_vote(v2);
        assert!(res.is_ok());
        let evidence = res.unwrap();
        assert!(evidence.is_some());
        
        let ev = evidence.unwrap();
        assert_eq!(ev.vote_a.voter, node);
        assert_eq!(ev.vote_a.proposal_id, "prop1");
        assert_eq!(ev.vote_b.proposal_id, "prop2");
    }
    
    #[test]
    fn test_diff_view_ok() {
        let mut registry = VoteRegistry::new();
        let node = NodeId("node1".into());
        
        let v1 = mock_vote("prop1", 1, ConsensusPhase::Prepare, node.clone(), Vote::Yes);
        registry.register_vote(v1).unwrap();
        
        // Different View -> OK
        let v2 = mock_vote("prop2", 2, ConsensusPhase::Prepare, node.clone(), Vote::Yes);
        let res = registry.register_vote(v2);
        assert!(res.is_ok());
        assert!(res.unwrap().is_none());
    }
}