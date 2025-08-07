use std::sync::Arc;

use crate::{
    cluster_proto::{
        Ack, 
        ProposalMessage
    },
    cluster::core::Cluster,
    env::proposal::Proposal
};

impl Cluster {
    /// Sends a proposal to a specific peer
    pub async fn submit_proposal(&self, proposal: Proposal) -> Result<Vec<Result<Ack, String>>, String> {
        let ack = self
            .local_env
            .write()
            .map_err(|_| "Failed to acquire write lock on local env")?
            .engine
            .submit_proposal(
                proposal, 
                Arc::clone(&self.network), 
            )
            .await
            .map_err(|e| format!("Failed to submit proposal: {}", e))?;
    
        Ok(ack)
    }

    pub fn handle_proposal(&mut self, msg: ProposalMessage) -> Result<Ack, String> {
        let proposal = Proposal::from_proto(msg).map_err(|e| format!("Failed to parse proposal: {}", e))?;
    
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
    
        let auth = match self.auth.read() {
            Ok(a) => a,
            Err(_) => {
                println!("⚠️ Failed to acquire read lock on auth");
                return Ok(Ack {
                    received: false,
                    message: "Falha ao adquirir lock de leitura em auth".into(),
                });
            }
        };
    
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
            if let Ok(mut env) = self.local_env.write() {
                env.engine.add_proposal(proposal.clone());
            } else {
                println!("⚠️ Failed to acquire write lock on local_env");
                return Ok(Ack {
                    received: false,
                    message: "Falha ao adquirir lock de escrita em local_env".into(),
                });
            }
    
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
    
}