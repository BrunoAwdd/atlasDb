use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use crate::{
    env::proposal::Proposal,
    network::{adapter::{ClusterMessage, NetworkAdapter}},
    peer_manager::PeerManager,
    utils::NodeId,
    Node,
};

use super::{
    evaluator::ConsensusEvaluator,
    pool::ProposalPool,
    registry::VoteRegistry,
    types::{ConsensusResult, Vote},
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
    pub fn new(peer_manager: Arc<RwLock<PeerManager>>, quorum_ratio: f64) -> Self {
        Self {
            peer_manager,
            pool: ProposalPool::new(),
            registry: VoteRegistry::new(),
            evaluator: ConsensusEvaluator::new(quorum_ratio),
        }
    }

    /// Adiciona uma proposta ao pool e inicializa registro de votos.
    pub fn add_proposal(&mut self, proposal: Proposal) {
        self.pool.add(proposal.clone());
        self.registry.register_proposal(&proposal.id);
    }

    /// Submete uma proposta e propaga aos peers pela rede.
    pub async fn submit_proposal(
        &mut self,
        proposal: Proposal,
        network: Arc<RwLock<dyn NetworkAdapter>>,
    ) -> Result<Vec<Result<crate::cluster_proto::Ack, String>>, String> {
        self.add_proposal(proposal.clone());

        let peers = {
            let pm = self.peer_manager.read().map_err(|_| "PeerManager lock failed")?;
            pm.get_active_peers().into_iter().collect::<Vec<_>>()
        };

        let mut results = Vec::new();
        let net = network.write().map_err(|_| "NetworkAdapter lock failed")?;

        for peer in peers {
            if peer == proposal.proposer {
                continue;
            }

            let peer_data = self
                .peer_manager
                .read()
                .unwrap()
                .get_peer_stats(&peer);

            if let Some(p) = peer_data {
                let ack = net.send_proposal(p, proposal.clone()).await
                    .map(|_| crate::cluster_proto::Ack {
                        received: true,
                        message: format!("Proposta recebida por {}", peer),
                    })
                    .map_err(|e| format!("Erro ao enviar para {}: {:?}", peer, e));

                results.push(ack);
            } else {
                results.push(Ok(crate::cluster_proto::Ack {
                    received: false,
                    message: format!("Peer {} não encontrado", peer),
                }));
            }
        }

        Ok(results)
    }

    /// Registra voto recebido de um peer.
    pub fn receive_vote(&mut self, vote_msg: crate::cluster_proto::VoteMessage) {
        let voter = NodeId(vote_msg.voter_id.clone());
        if !self.get_active_nodes().contains(&voter) {
            println!("⚠️ Ignorado voto de nó inativo: [{}]", voter);
            return;
        }

        match Vote::try_from(vote_msg.vote) {
            Ok(vote) => {
                self.registry.register_vote(&vote_msg.proposal_id, voter.clone(), vote.clone());
                println!("📥 [{}] votou {:?} na proposta [{}]", voter, vote, vote_msg.proposal_id);
            }
            Err(_) => println!("⚠️ Voto inválido ignorado: {}", vote_msg.vote),
        }
    }

    /// Propaga votos da proposta pela rede.
    pub async fn vote_proposals(
        &self,
        vote_batch: ClusterMessage,
        network: Arc<RwLock<dyn NetworkAdapter>>,
        proposer: &Node,
    ) -> Result<crate::cluster_proto::Ack, String> {
        let net = network.write().map_err(|_| "NetworkAdapter lock failed")?;
        if let Err(e) = net.send_votes(proposer.clone(), vote_batch).await {
            return Err(format!("Erro ao enviar votos: {:?}", e));
        }

        Ok(crate::cluster_proto::Ack {
            received: true,
            message: format!("Votos enviados por {}", proposer.id),
        })
    }

    /// Avalia todas as propostas e retorna os resultados.
    pub fn evaluate_proposals(&self) -> Vec<ConsensusResult> {
        self.evaluator
            .evaluate(&self.registry, &self.get_active_nodes())
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
    fn get_active_nodes(&self) -> HashSet<NodeId> {
        self.peer_manager
            .read()
            .expect("Falha ao ler PeerManager")
            .get_active_peers()
    }
}
