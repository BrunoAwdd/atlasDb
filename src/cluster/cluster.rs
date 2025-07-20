use std::{
    sync::{Arc, RwLock}, 
    time::{SystemTime, UNIX_EPOCH}
};

use tokio::sync::oneshot;

use crate::{
    cluster_proto::{
        HeartbeatMessage, 
        ProposalBatch, 
        ProposalMessage, 
        VoteBatch, 
        VoteMessage
    }, 
    env::{
        proposal::Proposal, 
        AtlasEnv
    }, 
    network::adapter::{
        ClusterMessage, 
        NetworkAdapter, 
        VoteData
    }, 
    peer_manager::{
        PeerCommand, 
        PeerManager
    }, 
    utils::NodeId
};
use super::node::Node;

use crate::cluster_proto::Ack;

// TODO: Implement timeouts for heartbeats
// TODO: Implement retry logic for fail
// TODO: Implement periodic health checks
// TODO: make new tests
// TODO: Implemente new metrics

/// Simulates a distributed cluster composed of multiple nodes.
///
/// This structure provides mechanisms for broadcasting messages,
/// simulating inter-node communication, and running cyclical simulations.
pub struct Cluster {
    /// The full set of nodes currently part of the cluster.
    pub local_env: Arc<RwLock<AtlasEnv>>,
    pub network: Arc<RwLock<dyn NetworkAdapter>>,
    pub local_node: Node,
    pub peer_manager: Arc<RwLock<PeerManager>>,
    pub shutdown_sender: Option<oneshot::Sender<()>>,
}

impl Cluster {
    /// Initializes a new, empty cluster.
    pub fn new(
        env: Arc<RwLock<AtlasEnv>>, 
        network: Arc<RwLock<dyn NetworkAdapter>>,
        node_id: NodeId,
    ) -> Self {
        let addr = network.read()
            .expect("Failed to acquire read lock")
            .get_address();

        let peer_manager = Arc::clone(&env.read().expect("Failed to acquire read lock").peer_manager);
        
        Cluster {
            local_env: env,
            network,
            local_node: Self::set_local_node(node_id, &addr),
            peer_manager,
            shutdown_sender: None,
        }
    }

    pub async fn vote_proposals(&mut self, votes: ClusterMessage, proposer_id: NodeId) -> Result<ClusterMessage, String> {
        let votes_batch: VoteBatch = match votes.clone() {
            ClusterMessage::VoteBatch { votes } => {
                let proto_votes: Vec<VoteMessage> = votes
                    .into_iter()
                    .map(|v| v.into_proto())
                    .collect();
        
                Ok::<VoteBatch, String>(VoteBatch { votes: proto_votes }) // <- aqui está o erro
            }
            _ => Err("ClusterMessage não é um VoteBatch.".into()),
        }?; // <- operador ? depende da tipagem

        let proposer = self.peer_manager
            .read()
            .map_err(|_| "Failed to lock peer manager")?
            .get_peer_stats(&proposer_id)
            .ok_or_else(|| format!("Proposer node {} not found", proposer_id))?;

        println!("🚀 Votes sent (BG): {:?}", self.local_node.id);
    
        self.local_env
            .write()
            .map_err(|_| "Failed to acquire write lock on local env")?
            .engine
            .vote_proposals(
                votes_batch, 
            Arc::clone(&self.network), 
                &proposer
            ) 
            .await
            .map_err(|e| format!("Erro ao votar propostas: {}", e))?;

        println!("🚀 Votes sent (ED): {:?}", self.local_node.id);
    
        Ok(votes)
    }

    pub fn shutdown_grpc(&mut self) {
        if let Some(tx) = self.shutdown_sender.take() {
            let _ = tx.send(()); // envia sinal para parar o servidor
            println!("🛑 Enviando sinal de shutdown para gRPC.");
        }
    }

    fn set_local_node(id: NodeId, addr: &str) -> Node {
        Node::new(id.into(), addr.to_string(), None, 0.0)
    }

    /// Adds a new node to the cluster by its unique identifier.
    pub fn add_node(&mut self, id: NodeId, stats: Node) -> Result<(), String> {
        let cmd = PeerCommand::Register(id, stats);
        let mut manager = self.peer_manager.write()
            .map_err(|_| "Failed to acquire write lock on peer manager")?;
        manager.handle_command(cmd);
        Ok(())
    }

    /// Broadcasts heartbeat messages from all nodes to all other peers.
    pub async fn broadcast_heartbeats(&self) -> Result<(), String> {
        let peers = {
            let manager = self.peer_manager.read()
                .map_err(|_| "Failed to acquire read lock on peer manager")?;
            manager.get_active_peers().iter().cloned().collect::<Vec<NodeId>>()
        };
        
        let sender_id = self.local_node.id.clone();
        let mut errors = Vec::new();

        for peer_id in peers {
            if peer_id != sender_id {
                if let Err(e) = self.send_heartbeat(peer_id.clone(), "broadcast".to_string()).await {
                    errors.push(format!("Failed to send heartbeat to {}: {}", peer_id, e));
                }
            }
        }

        if !errors.is_empty() {
            return Err(format!("Some heartbeats failed: {}", errors.join(", ")));
        }
        
        Ok(())
    }

    pub async fn send_heartbeat(&self, to: NodeId, msg: String) -> Result< Ack, String> {
        let ack = self
            .local_env
            .write()
            .map_err(|_| "Failed to acquire write lock on local env")?
            .send_heartbeat(self.local_node.clone(), &to)
            .await
            .map_err(|e| format!("Failed to send heartbeat: {}", e))?;

        Ok(ack)
    }

    /// Sends a heartbeat message to a specific peer
    pub async fn submit_proposal(&self, proposal: Proposal) -> Result<Ack, String> {
        println!("🚀 Submetendo proposta: {:?}", proposal);
        let ack = self
            .local_env
            .write()
            .map_err(|_| "Failed to acquire write lock on local env")?
            .submit_proposal(&proposal, self.local_node.id.clone())
            .await
            .map_err(|e| format!("Failed to submit proposal: {}", e))?;
    
        Ok(ack)
    }
    
    /// Handles incoming heartbeat messages
    pub fn handle_heartbeat(&self, msg: HeartbeatMessage) -> Ack {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        println!("⏱️ Heartbeat recebido de [{}] em [{}]", msg.from, msg.timestamp);
        
        // Update peer's last seen timestamp
        if let Ok(mut manager) = self.peer_manager.write() {
            // Note: This would need a method to update last_seen in PeerManager
            // manager.update_peer_last_seen(&msg.from, timestamp);
        }
        
        Ack {
            received: true,
            message: format!("ACK recebido por {} em {}", self.local_node.id, timestamp),
        }
    }

    pub fn handle_vote_batch(&mut self, msg: VoteBatch) -> Result<Ack, String> {
        let votes = msg.votes
            .into_iter()
            .map(|v| VoteData::from_proto(v))
            .collect();

        println!("?🚀? I am: {:?}", self.local_node);

        self.local_env.write().map_err(|_| "Failed to acquire write lock on local env")?.engine.receive_vote_batch(votes, self.local_node.id.clone());

        Ok(Ack {
            received: true,
            message: format!("Vote batch received by {}", self.local_node.id),
        })
    }

    pub fn handle_proposal_batch(&mut self, msg: ProposalBatch) -> Result<Ack, String>  {
        let proposals: Vec<Proposal> = msg.proposals.into_iter().map(|p| Proposal::from_proto(p)).collect();

        for proposal in proposals {
            self.local_env.write().map_err(|_| "Failed to acquire write lock on local env")?.engine.add_proposal(proposal);
        }

        Ok(Ack {
            received: true,
            message: format!("Proposal batch received by {}", self.local_node.id),
        })
    }

    pub fn handle_proposal(&mut self, msg: ProposalMessage) -> Result<Ack, String>  {
        let proposal = Proposal::from_proto(msg);

        println!("🚀 Proposta recebida: {:?}, node_id: {}", proposal, self.local_node.id);

        self.local_env.write().map_err(|_| "Failed to acquire write lock on local env")?.engine.add_proposal(proposal.clone());

        Ok(Ack {
            received: true,
            message: format!("Proposta {} recebida por {}", proposal.id, self.local_node.id),
        })
    }

    /// Gets the number of active peers in the cluster
    pub fn get_peer_count(&self) -> Result<usize, String> {
        let manager = self.peer_manager.read()
            .map_err(|_| "Failed to acquire read lock on peer manager")?;
        Ok(manager.get_active_peers().len())
    }

    /// Checks if a specific peer is active
    pub fn is_peer_active(&self, peer_id: &NodeId) -> Result<bool, String> {
        let manager = self.peer_manager.read()
            .map_err(|_| "Failed to acquire read lock on peer manager")?;
        Ok(manager.get_peer_stats(peer_id).is_some())
    }

    pub fn clone(&self) -> Self {
        println!("!!!!!!!!!!!!!!!! Cluster Clonado !!!!!!!!!!!!!!!!");
        Cluster {
            local_env: self.local_env.clone(),
            network: Arc::clone(&self.network),
            local_node: self.local_node.clone(),
            peer_manager: Arc::clone(&self.peer_manager),
            shutdown_sender: None,
        }
    }
}
