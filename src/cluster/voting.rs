use std::sync::Arc;

use crate::{
    cluster::core::Cluster,
    cluster_proto::{Ack, VoteBatch, VoteMessage},
    network::adapter::{ClusterMessage, VoteData},
    utils::NodeId,
};

impl Cluster {
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
}
