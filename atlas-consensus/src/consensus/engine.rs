use std::{
    collections::HashSet,
    sync::{Arc},
};
use tokio::sync::{RwLock};
use tracing::{info, warn};

use atlas_common::{
    env::consensus::types::ConsensusResult,
    env::proposal::Proposal,
    utils::NodeId, // Keep NodeId as it's used in get_active_nodes
};

use atlas_p2p::PeerManager;
use atlas_common::env::vote_data::VoteData;

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
    pub pending_evidence: Vec<atlas_common::env::consensus::evidence::EquivocationEvidence>,
}

impl ConsensusEngine {
    pub fn new(peer_manager: Arc<RwLock<PeerManager>>, policy: QuorumPolicy) -> Self {
        Self {
            peer_manager,
            pool: ProposalPool::new(),
            registry: VoteRegistry::new(),
            evaluator: ConsensusEvaluator::new(policy),
            pending_evidence: Vec::new(),
        }
    }

    /// Adiciona uma proposta ao pool e inicializa registro de votos.
    pub(crate) fn add_proposal(&mut self, proposal: Proposal) {
        self.pool.add(proposal.clone());
        self.registry.register_proposal(&proposal.id);
    }

    /// Remove uma proposta do pool.
    pub(crate) fn remove_proposal(&mut self, id: &str) {
        self.pool.remove(id);
    }
    
    /// Registra voto recebido de um peer.
    /// Retorna Evidence se detectar comportamento malicioso.
    pub(crate) async fn receive_vote(&mut self, vote_msg: VoteData) -> Option<atlas_common::env::consensus::evidence::EquivocationEvidence> {
        let voter = vote_msg.voter.clone();
        if !self.get_active_nodes().await.contains(&voter) {
            warn!("⚠️ Ignorado voto de nó inativo: [{}]", vote_msg.voter.clone());
            return None;
        }

        match self.registry.register_vote(vote_msg.clone()) {
            Ok(Some(evidence)) => {
                warn!("🚨 MALICIOUS BEHAVIOR DETECTED: Node {} committed equivocation!", voter);
                self.pending_evidence.push(evidence.clone());
                Some(evidence)
            },
            Ok(None) => {
                info!("📥 [{}] votou {:?} na proposta [{}] (Fase: {:?})", voter, vote_msg.vote, vote_msg.proposal_id, vote_msg.phase);
                None
            },
            Err(e) => {
                warn!("⚠️ Erro ao registrar voto: {}", e);
                None
            },
        }
    }

    /// Avalia todas as propostas e retorna os resultados.
    pub(crate) async fn evaluate_proposals(&mut self, ledger: &atlas_ledger::Ledger) -> Vec<ConsensusResult> {
        // 1. Process Pending Evidence (Slashing)
        if !self.pending_evidence.is_empty() {
            info!("⚖️ Processing {} pending evidences for slashing...", self.pending_evidence.len());
            
            // Clone to iterate and clear original
            let evidences: Vec<_> = self.pending_evidence.drain(..).collect();
            
            for evidence in evidences {
                // Convert NodeId to Address
                if let Some(address) = atlas_p2p::utils::node_id_to_address(&evidence.offender().0) {
                     info!("⚔️ SLASHING VALIDATOR {} for Double Voting (View {})", address, evidence.vote_a.view);
                     // 100% Slashing for Equivocation (Severe)
                     // Or just a fixed penalty? Let's do 1,000,000 ATLAS or All.
                     // Typically huge.
                     if let Err(e) = ledger.slash_validator(&address, 1_000_000).await {
                         warn!("❌ Failed to slash validator {}: {}", address, e);
                     }
                } else {
                    warn!("⚠️ Cannot slash node {}: Address conversion failed.", evidence.offender().0);
                }
            }
        }

        self.evaluator
            .evaluate_weighted(&self.registry, &self.get_active_nodes().await, ledger).await
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
