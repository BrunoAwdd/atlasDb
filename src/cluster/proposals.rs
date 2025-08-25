use std::sync::Arc;

use crate::{
    cluster_proto::{
        Ack, 
        ProposalMessage
    }, 
    cluster::core::Cluster, 
    env::proposal::Proposal, 
    NodeId
};

impl Cluster {
    pub(super) async fn add_proposal(&self, proposal: Proposal) -> Result<(), String> {
        self.local_env.engine.lock().await
            .add_proposal(proposal.clone());

        Ok(())
    }

    pub(super) async fn broadcast_proposals(&self) -> Result<(), String> {
        let peers = {
            let manager = self.peer_manager.read().await;
            manager.get_active_peers().iter().cloned().collect::<Vec<NodeId>>()
        };

        let sender_id = self.local_node.id.clone();
        let mut errors = Vec::new();
        let proposals = self.local_env.engine.lock()
            .await            
            .pool
            .all()
            .clone();


        // Ao inves de loop em proposal, vou enviar todas
        for (_, proposal) in proposals {
            for peer_id in &peers {
                if peer_id != &sender_id {
                    if let Err(e) = self.submit_proposal(proposal.clone()).await {
                        errors.push(format!("Failed to submit proposal to {}: {}", peer_id, e));
                    }
                }
            }
        }

        Ok(())
    }

    /// Sends a proposal to a specific peer
    async fn submit_proposal(&self, proposal: Proposal) -> Result<Vec<Result<Ack, String>>, String> {
        let ack = self
            .local_env
            .engine
            .lock()
            .await
            .submit_proposal(
                proposal, 
                Arc::clone(&self.network), 
            )
            .await
            .map_err(|e| format!("Failed to submit proposal: {}", e))?;
    
        Ok(ack)
    }

    pub(super) async fn handle_proposal(&self, msg: ProposalMessage) -> Result<Ack, String> {
        let proposal = Proposal::from_proto(msg.clone()).map_err(|e| format!("Failed to parse proposal: {}", e))?;

        let bytes = match bincode::serialize(&proposal.content) {
            Ok(b) => b,
            Err(e) => {
                println!("⚠️ Failed to serialize proposal: {}", e);
                return Ok(Ack {
                    received: false,
                    message: format!("Falha ao serializar proposta: {}", e),
                });
            }
        };

        let auth = self.auth.read().await;
        
        
        let is_valid = match auth.verify(bytes, &proposal.signature) {
            Ok(valid) => valid,
            Err(e) => {
                println!("⚠️ Failed to verify signature: {}", e);
                return Ok(Ack {
                    received: false,
                    message: format!("Assinatura inválida: {}", e),
                });
            }
        };
    
        if is_valid {
            self.local_env.engine.lock().await
                .add_proposal(proposal.clone());
    
            Ok(Ack {
                received: true,
                message: format!("Proposta {} recebida por {}", proposal.id, self.local_node.id),
            })
        } else {
            println!("⚠️ Failed to verify signature");
            Ok(Ack {
                received: false,
                message: format!("Assinatura da proposta {} inválida", proposal.id),
            })
        }
    }

    pub(super) async fn evaluate_proposals(&self) -> Result<(), String> {
        println!("🗳️ Avaliando consenso");
        self.local_env.engine.lock().await.evaluate_proposals().await;
        Ok(())
    }
    
}