use std::sync::Arc;

use crate::{
    cluster::core::Cluster,
    cluster_proto::{Ack, VoteMessage},
    network::adapter::{ClusterMessage, VoteData},
    utils::NodeId, Vote,
};

impl Cluster {
    pub(super) async fn vote_proposals(&self) -> Result<(), String> {
        let engine_guard = self.local_env.engine.lock().await;
        let proposal_pool = engine_guard.get_all_proposals().all().clone();
        drop(engine_guard);

        for (_, proposal) in proposal_pool{
            let content = proposal.content.clone();
            let serialized = bincode::serialize(&content).unwrap();

            let is_valid = self.auth.read().await.verify(serialized, &proposal.signature);

            let vote = match is_valid {
                Ok(true) => Vote::Yes,
                Ok(false) => Vote::No,
                Err(_e) => Vote::Abstain
            };

            let vote_to_sign = VoteData {        
                proposal_id: proposal.id.clone(),
                vote: vote,
                voter: self.local_node.id.clone(),
            };

            let vote_serialized = bincode::serialize(&vote_to_sign).unwrap();
            let signed_vote = self.auth.read().await.sign(vote_serialized.clone(), "12345".to_string())?;
    
            let vote_data = vote_to_sign.into_cluster_message(Vec::new(), signed_vote);

            self.vote_proposal(vote_data, proposal.proposer).await?;

            println!("Sending Votes...");
        }


        Ok(())
    }

    async fn vote_proposal(&self, vote: ClusterMessage, proposer_id: NodeId) -> Result<(), String> {    
        // Busca o proposer
        let proposer = self.peer_manager
            .read()
            .await
            .get_peer_stats(&proposer_id)
            .ok_or_else(|| format!("Proposer node {} not found", proposer_id))?;
        
        self.local_env
            .engine
            .lock()
            .await
            .vote_proposals(
                vote,
                Arc::clone(&self.network),
                &proposer,
            )
            .await
            .map_err(|e| format!("Erro ao votar propostas: {}", e))?;
    
        // Retorna o ClusterMessage original (já validado como Vote)
        Ok(())
    }
    
    pub(super) async fn handle_vote(&self, msg: VoteMessage) -> Result<Ack, String> {
        let vote_data = VoteData::from_proto(msg.clone());
        let vote_serialized = bincode::serialize(&vote_data).unwrap();

        let signature_array: [u8; 64] = msg.signature
            .as_slice()
            .try_into()
            .map_err(|_| "Assinatura com tamanho inválido")?;

        let auth = self.auth.read().await;

        let is_valid = match auth.verify(vote_serialized, &signature_array) {
            Ok(valid) => valid,
            Err(e) => {
                return Ok(Ack {
                    received: false,
                    message: format!("Assinatura inválida: {}", e),
                });
            }
        };
        drop(auth);

        let engine = self.local_env.engine.lock().await;
        let votes = engine.get_all_votes().clone(); // clona os dados para sair do guard
        drop(engine); // opcional: solta o lock antes de usar votes

        println!("Votes {} {:?}", self.local_node.id, &votes);


        if is_valid {
            self.local_env.engine.lock().await.receive_vote(msg.clone()).await;
    
            Ok(Ack {
                received: true,
                message: format!("Votos {} recebidos por {}", msg.proposal_id.clone(), self.local_node.id),
            })
        } else {
            Ok(Ack {
                received: false,
                message: format!("Votos {} inválidos", msg.proposal_id),
            })
        }
    }
}
