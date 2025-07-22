//! consensus.rs
//!
//! Asynchronous consensus simulation engine with probabilistic voting and quorum evaluation.
//!
//! This module simulates the core logic of a distributed consensus protocol,
//! where nodes vote independently on proposals and quorum is used to determine acceptance.
//!
//! The engine is deliberately asynchronous, failure-tolerant, and latency-aware,
//! serving as a conceptual foundation rather than a production-grade implementation.

use std::{
    collections::{
        HashMap, 
        HashSet
    }, 
    fmt, 
    sync::{Arc, RwLock}
};
use serde::{Serialize, Deserialize};
use crate::{
    cluster_proto::{
        Ack, 
        ProposalBatch, 
        VoteBatch}
        , 
    network::adapter::{
        ClusterMessage, 
        VoteData
    }, 
    NetworkAdapter, Node
};

use super::{
    super::{peer_manager::PeerManager, utils::NodeId},
    proposal::Proposal
};

/// Represents a binary vote from a node regarding a proposal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Vote {
    Yes,
    No,
    Abstain
}

impl From<Vote> for i32 {
    fn from(v: Vote) -> Self {
        match v {
            Vote::Yes => 0,
            Vote::No => 1,
            Vote::Abstain => 2,
        }
    }
}

impl std::convert::TryFrom<i32> for Vote {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Vote::Yes),
            1 => Ok(Vote::No),
            2 => Ok(Vote::Abstain),
            _ => Err(()),
        }
    }
}

/// The result of consensus evaluation for a single proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusResult {
    /// Whether the proposal reached quorum and was approved.
    pub approved: bool,

    /// The number of affirmative (Yes) votes received.
    pub votes_received: usize,

    /// The proposal ID this result corresponds to.
    pub proposal_id: String,
}

/// The core asynchronous consensus engine.
///
/// Manages the lifecycle of proposals, tracks votes,
/// and determines approval based on a quorum threshold.
#[derive(Debug, Clone)]
pub struct ConsensusEngine {
    pub peer_manager: Arc<RwLock<PeerManager>>,
    pub proposals: Vec<Proposal>,
    pub votes: HashMap<String, HashMap<NodeId, Vote>>,
    pub quorum_ratio: f64,
}

impl ConsensusEngine {
    /// Initializes a new consensus engine with a given cluster size.
    ///
    /// Quorum is calculated as majority: (n / 2) + 1
    pub fn new(peer_manager: Arc<RwLock<PeerManager>>, quorum_ratio: f64) -> Self {
        Self {
            proposals: Vec::new(),
            votes: HashMap::new(),
            quorum_ratio,
            peer_manager,
        }
    }

    // Add Proposal
    pub fn add_proposal(&mut self, proposal: Proposal) -> () {
        self.proposals.push(proposal.clone());

        println!("🚀 Proposta adicionada: {:?}", self.proposals);
    }


    /// Submits a new proposal to the consensus engine.
    ///
    /// The proposal is tracked and awaits votes from peer nodes.
    pub async fn submit_proposal(
        &mut self,
        proposal: Proposal,
        network: Arc<RwLock<dyn NetworkAdapter>>,
        local_node_id: NodeId,
    ) -> Result<Ack, String> {
        // 1. Processa localmente
        println!("🚀 Submetendo proposta local: {:?}", proposal);

        self.proposals.push(proposal.clone());
        self.votes.insert(proposal.id.clone(), HashMap::new());

        println!("🚀 Votes started(Start) {}: {:?}", local_node_id, self.votes);
        println!("🚀 Proposals started(Start): {:?}", self.proposals);
    
        // 2. Propaga para outros peers
        let peers = {
            let manager = self.peer_manager.read()
                .map_err(|_| "Failed to acquire read lock on peer manager")?;
            manager.get_active_peers().iter().cloned().collect::<Vec<NodeId>>()
        };

        println!("📡 Enviando proposta para peers: {:?}", peers);
    
        let network = network.write()
            .map_err(|_| "Failed to acquire write lock on network adapter")?;
    
        let mut errors = Vec::new();
    
        for peer_id in peers {
            if peer_id != local_node_id {
                let msg = ClusterMessage::Proposal {
                    proposal: proposal.clone(),
                    public_key: vec![],
                    signature: vec![],
                };

                let node = self.peer_manager.read()
                    .map_err(|_| "Failed to acquire read lock on peer manager")?
                    .get_peer_stats(&peer_id)
                    .ok_or_else(|| format!("Peer {} not found", peer_id))?;

                if let Err(e) = network.send_to(node, msg).await {
                    errors.push(format!("Erro ao enviar para {}: {:?}", peer_id, e));
                }
            }
        }
    
        if !errors.is_empty() {
            println!("⚠️ Alguns envios falharam: {:?}", errors);
        } else {
            println!("✅ Proposta propagada para todos os peers");
        }
    
        Ok(Ack {
            received: true,
            message: format!("Proposta {} recebida por {}", proposal.id, local_node_id),
        })
    }

    pub async fn vote_proposals(
        &mut self,
        vote_batch: VoteBatch,
        network: Arc<RwLock<dyn NetworkAdapter>>,
        proposer: &Node,
    ) -> Result<Ack, String>  {
        let peers = {
            let manager = self.peer_manager.read()
                .map_err(|_| "Failed to acquire read lock on peer manager")?;
            manager.get_active_peers().iter().cloned().collect::<Vec<NodeId>>()
        };

        println!("📡 Enviando votos para proposer: {:?}", peers);
    
        let network = network.write()
            .map_err(|_| "Failed to acquire write lock on network adapter")?;
    
        let mut errors = Vec::new();

        if let Err(e) = network.send_votes_batch(proposer.clone(), vote_batch).await {
            errors.push(format!("Erro ao enviar para {}: {:?}", proposer.id, e));
        }

        if !errors.is_empty() {
            println!("⚠️ Alguns envios falharam: {:?}", errors);
        } else {
            println!("✅ Proposta propagada para todos os peers");
        }
    
        Ok(Ack {
            received: true,
            message: format!("Vote batch sent by {}", proposer.id),
        })
    }

    pub fn receive_vote_batch(&mut self, votes: Vec<VoteData>, local_node_id: NodeId) {
        votes
            .into_iter()
            .for_each(|vote| 
                self.receive_vote(
                    &vote.proposal_id, 
                    vote.voter, 
                    vote.vote,
                    local_node_id.clone()
            ));
    }

    /// Registers a vote from a peer node on a specific proposal.
    ///
    /// This method simulates asynchronous, out-of-order voting from the network.
    pub fn receive_vote(&mut self, proposal_id: &str, from_node: NodeId, vote: Vote, local_node_id: NodeId) {
        if !self.get_active_nodes().contains(&from_node) {
            println!("⚠️ Ignored vote from unknown or inactive node: [{}]", from_node);
            return;
        }

        if let Some(voters) = self.votes.get_mut(proposal_id) {
            voters.insert(from_node.clone(), vote.clone());

            println!(
                "📥 [{}] voted {:?} on proposal [{}] (Confirmed)",
                from_node, vote, proposal_id
            );
        }
        
        println!("🚀 Votes started (end) {}: {:?}", local_node_id, self.votes);
        println!("🚀 Proposals started(end): {:?}", self.proposals);
    }

    /// Evaluates all tracked proposals and computes consensus results.
    ///
    /// Proposals are considered approved if they receive quorum (≥ majority) of `Yes` votes.
    pub fn evaluate_proposals(&self) -> Vec<ConsensusResult> {
        let quorum_count = (self.get_active_nodes().len() as f64 * self.quorum_ratio/self.quorum_ratio).ceil() as usize;

        println!("🗳️ Quorum: Active: {} Ratio: {} Count: {}", self.get_active_nodes().len(), self.quorum_ratio, quorum_count);

        let mut results = Vec::new();

        for (id, voters) in &self.votes {
            let yes_votes = voters.values().filter(|v| matches!(v, Vote::Yes)).count();
            let approved = yes_votes >= quorum_count;

            results.push(ConsensusResult {
                approved,
                votes_received: yes_votes,
                proposal_id: id.clone(),
            });

            println!(
                "🗳️ Proposal [{}] received {}/{} YES votes — {}",
                id,
                yes_votes,
                quorum_count,
                if approved { "APPROVED ✅" } else { "REJECTED ❌" }
            );
        }

        results
    }

    pub fn update_active_nodes(&mut self) {
        println!("Not implemented");
    }

    fn get_active_nodes(&self) -> HashSet<NodeId> {
        self.peer_manager.read().expect("Failed to acquire read lock").get_active_peers()
    }
}
