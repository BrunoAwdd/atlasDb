use std::sync::Arc;

use crate::{
    cluster_proto::{
        Ack, 
        ProposalBatch, 
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
                self.local_node.id.clone()
            )
            .await
            .map_err(|e| format!("Failed to submit proposal: {}", e))?;
    
        Ok(ack)
    }

    pub async fn submit_proposal_batch(&self, proposals: Vec<Proposal>, public_key: Vec<u8>, signature: Vec<u8>) -> Result<Ack, String> {
        let proposal_batch = ProposalBatch { 
            proposals: proposals
                .into_iter()
                .map(|p| p.into_proto()).collect(),
            public_key,
            signature: signature.clone(),
            };

        let ack = self
            .local_env
            .write()
            .map_err(|_| "Failed to acquire write lock on local env")?
            .engine
            .submit_proposal_batch(
                proposal_batch, 
                Arc::clone(&self.network),
                &self.local_node
            )
            .await
            .map_err(|e| format!("Failed to submit proposal batch: {}", e))?;
    
        Ok(ack)
    }
    
    pub fn handle_proposal_batch(&mut self, msg: ProposalBatch) -> Result<Ack, String>  {
        let proposals: Vec<Proposal> = msg.proposals.into_iter().map(|p| Proposal::from_proto(p)).collect();

        let message: Vec<u8> = bincode::serialize(&proposals).map_err(|e| format!("Failed to serialize proposals: {}", e))?;

        match self.auth.read().map_err(|_| "Failed to acquire read lock on auth")?.verify(message, msg.signature.clone()) {
            Ok(true) => {}
            Ok(false) => return Err("Proposals not authorized".into()),
            Err(e) => return Err(format!("Failed to verify proposals: {}", e)),
        }

        for proposal in proposals {
            self
                .local_env
                .write()
                .map_err(|_| "Failed to acquire write lock on local env")?
                .engine
                .add_proposal(proposal);
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
}