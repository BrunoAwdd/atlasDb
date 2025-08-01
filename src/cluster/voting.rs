use std::sync::Arc;

use crate::{
    cluster::core::Cluster,
    cluster_proto::{Ack, VoteMessage},
    network::adapter::{ClusterMessage, VoteData},
    utils::NodeId,
};

impl Cluster {
    pub async fn vote_proposals(&mut self, vote: ClusterMessage, proposer_id: NodeId) -> Result<(), String> {    
        // Busca o proposer
        let proposer = self.peer_manager
            .read()
            .map_err(|_| "Failed to lock peer manager")?
            .get_peer_stats(&proposer_id)
            .ok_or_else(|| format!("Proposer node {} not found", proposer_id))?;
        
        self.local_env
            .write()
            .map_err(|_| "Failed to acquire write lock on local env")?
            .engine
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
    

    pub fn handle_vote(&mut self, msg: VoteMessage) -> Result<Ack, String> {

        let vote_data = VoteData::from_proto(msg.clone());
        //let vote_json = serde_json::to_string(&vote_data).map_err(|e| e.to_string()).unwrap();
        let vote_serialized = bincode::serialize(&vote_data).unwrap();

        let signature_array: &[u8; 64] = msg.signature
            .as_slice()
            .try_into()
            .map_err(|_| "Assinatura com tamanho inválido")?;

        //println!("📥 Vote recebido: {}", vote_json);


        let auth = match self.auth.read() {
            Ok(a) => a,
            Err(_) => {
                return Ok(Ack {
                    received: false,
                    message: "Falha ao adquirir lock de leitura em auth".into(),
                });
            }
        };

        let is_valid = match auth.verify(vote_serialized, signature_array) {
            Ok(valid) => valid,
            Err(e) => {
                return Ok(Ack {
                    received: false,
                    message: format!("Assinatura inválida: {}", e),
                });
            }
        };

        if is_valid {
            if let Ok(mut env) = self.local_env.write() {
                env.engine.receive_vote(msg.clone());
            } else {
                return Ok(Ack {
                    received: false,
                    message: "Falha ao adquirir lock de escrita em local_env".into(),
                });
            }
    
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
