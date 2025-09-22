
use tokio::sync::mpsc;

use crate::{
    cluster::core::Cluster, 
    env::proposal::Proposal, 
    network::p2p::adapter::AdapterCmd,
};

impl Cluster {
    pub(super) async fn add_proposal(&self, proposal: Proposal) -> Result<(), String> {
        self.local_env.engine.lock().await
            .add_proposal(proposal.clone());

        Ok(())
    }

    pub(crate) async fn get_proposals(&self) -> Result<Vec<Proposal>, String> {
        let proposals = self.local_env.engine.lock().await.pool.all().clone();
        Ok(proposals.values().cloned().collect())
    }

    pub(crate) async fn handle_proposal(&self, bytes: Vec<u8>) -> Result<(), String> {
        let proposal: Proposal = bincode::deserialize(&bytes)
            .map_err(|e| format!("decode proposal: {e}"))?;

        println!("📩 Proposta recebida: {:?}", proposal);

        // bytes canônicos para assinatura
        let sign_bytes = crate::env::proposal::signing_bytes(&proposal);
        let ok = self.auth.read().await
            .verify(sign_bytes, &proposal.signature)
            .map_err(|e| format!("verify failed: {e}"))?;
        if !ok { return Err(format!("assinatura inválida para {}", proposal.id)); }

        self.local_env.engine.lock().await.add_proposal(proposal);
        Ok(())
    }

    pub(super) async fn evaluate_proposals(&self) -> Result<(), String> {
        println!("🗳️ Avaliando consenso");
        self.local_env.engine.lock().await.evaluate_proposals().await;
        Ok(())
    }
    
}