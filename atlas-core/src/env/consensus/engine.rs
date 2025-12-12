use std::{
    collections::HashSet,
    sync::{Arc},
};
use tokio::sync::{RwLock};
use tracing::{info, warn};

use atlas_common::{
    utils::NodeId,
    env::consensus::types::{Vote, ConsensusResult},
};

use crate::{
    env::{
        proposal::Proposal, vote_data::VoteData
    },  
    peer_manager::PeerManager, 
};

use super::{
    evaluator::{ConsensusEvaluator, QuorumPolicy},
    pool::ProposalPool,
    registry::VoteRegistry,
};

/// Motor de consenso assíncrono e modular.
#[derive(Debug, Clone)]
pub struct ConsensusEngine {
    pub peer_manager: Arc<RwLock<PeerManager>>,
    pub pool: ProposalPool,
    pub registry: VoteRegistry,
    pub evaluator: ConsensusEvaluator,
}

impl ConsensusEngine {
    pub fn new(peer_manager: Arc<RwLock<PeerManager>>, policy: QuorumPolicy) -> Self {
        Self {
            peer_manager,
            pool: ProposalPool::new(),
            registry: VoteRegistry::new(),
            evaluator: ConsensusEvaluator::new(policy),
        }
    }

    /// Adiciona uma proposta ao pool e inicializa registro de votos.
    pub(crate) fn add_proposal(&mut self, proposal: Proposal) {
        self.pool.add(proposal.clone());
        self.registry.register_proposal(&proposal.id);
    }
    
    /// Registra voto recebido de um peer.
    pub(crate) async fn receive_vote(&mut self, vote_msg: VoteData) {
        let voter = vote_msg.voter.clone();
        if !self.get_active_nodes().await.contains(&voter) {
            warn!("⚠️ Ignorado voto de nó inativo: [{}]", vote_msg.voter.clone());
            return;
        }

        match Vote::try_from(vote_msg.vote.clone()) {
            Ok(vote) => {
                match self.registry.register_vote(&vote_msg.proposal_id, vote_msg.phase.clone(), voter.clone(), vote.clone()) {
                    Ok(_) => info!("📥 [{}] votou {:?} na proposta [{}] (Fase: {:?})", voter, vote, vote_msg.proposal_id, vote_msg.phase),
                    Err(e) => warn!("🚨 EQUIVOCATION DETECTED: {}", e),
                }
            }
            Err(_) => warn!("⚠️ Voto inválido ignorado: {}", vote_msg.vote.to_string()),
        }
    }

    /// Avalia todas as propostas e retorna os resultados.
    pub(crate) async fn evaluate_proposals(&self) -> Vec<ConsensusResult> {
        self.evaluator
            .evaluate(&self.registry, &self.get_active_nodes().await)
    }

    /// Expõe os votos internamente (por exemplo, para salvar ou auditar).
    pub fn get_all_votes(&self) -> &VoteRegistry {
        &self.registry
    }

    /// Expõe todas as propostas.
    pub fn get_all_proposals(&self) -> &ProposalPool {
        &self.pool
    }

    /// Expõe os nós ativos (com leitura protegida).
    async fn get_active_nodes(&self) -> HashSet<NodeId> {
        self.peer_manager
            .read()
            .await
            .get_active_peers()
    }
}
