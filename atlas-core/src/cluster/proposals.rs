use crate::{cluster::core::Cluster, env::proposal::Proposal, network::p2p::adapter::AdapterCmd, error::{AtlasError, Result}};
use tracing::{info, warn};

const PROPOSAL_TOPIC: &str = "atlas/proposal/v1";

impl Cluster {
    /// Prepara e retorna um comando de publicação para uma nova proposta.
    ///
    /// Esta função adiciona a proposta ao pool de consenso local, a serializa
    /// e, em seguida, retorna um `AdapterCmd::Publish` que pode ser enviado
    /// pela camada de rede para disseminar a proposta via gossip.
    pub async fn submit_proposal(&self, proposal: Proposal) -> Result<AdapterCmd> {
        // 1. Adicionar a proposta ao nosso próprio pool de consenso primeiro.
        self.add_proposal(proposal.clone()).await?;

        // 2. Serializar a proposta para enviar pela rede.
        let bytes = bincode::serialize(&proposal)
            .map_err(|e| AtlasError::Other(format!("failed to serialize proposal: {e}")))?;

        // 3. Criar e retornar o comando para publicação, delegando o envio.
        Ok(AdapterCmd::Publish {
            topic: PROPOSAL_TOPIC.into(),
            data: bytes,
        })
    }

    pub(super) async fn add_proposal(&self, proposal: Proposal) -> Result<()> {
        self.local_env.engine.lock().await
            .add_proposal(proposal.clone());

        self.local_env.storage.write().await.log_proposal(proposal);

        Ok(())
    }

    pub(crate) async fn get_proposals(&self) -> Result<Vec<Proposal>> {
        let proposals = self.local_env.engine.lock().await.pool.all().clone();
        Ok(proposals.values().cloned().collect())
    }

    pub(crate) async fn handle_proposal(&self, bytes: Vec<u8>) -> Result<()> {
        let proposal: Proposal = bincode::deserialize(&bytes)
            .map_err(|e| AtlasError::Other(format!("decode proposal: {e}")))?;

        info!("📩 Proposta recebida: {:?}", proposal);
        tracing::info!(target: "consensus", "EVENT:RECEIVE_PROPOSAL id={} from={}", proposal.id, proposal.proposer);

        // bytes canônicos para assinatura
        let sign_bytes = crate::env::proposal::signing_bytes(&proposal);
        let ok = self.auth.read().await
            .verify_with_key(sign_bytes, &proposal.signature, &proposal.public_key)
            .map_err(|e| AtlasError::Auth(format!("verify failed: {e}")))?;
        
        if !ok { 
            warn!("❌ Assinatura INVÁLIDA para proposta {}", proposal.id);
            tracing::warn!(target: "consensus", "EVENT:VERIFY_PROPOSAL_FAIL id={}", proposal.id);
            return Err(AtlasError::Auth(format!("assinatura inválida para {}", proposal.id))); 
        }

        info!("✅ Assinatura verificada com sucesso para proposta {} (Proposer: {})", proposal.id, proposal.proposer);
        tracing::info!(target: "consensus", "EVENT:VERIFY_PROPOSAL_OK id={}", proposal.id);

        self.add_proposal(proposal).await?;
        Ok(())
    }

    pub(crate) async fn evaluate_proposals(&self) -> Result<Vec<atlas_sdk::env::consensus::types::ConsensusResult>> {
        info!("🗳️ Avaliando consenso");
        let results = self.local_env.engine.lock().await.evaluate_proposals().await;
        Ok(results)
    }
    
    pub(crate) async fn commit_proposal(&self, result: atlas_sdk::env::consensus::types::ConsensusResult) -> Result<()> {
        info!("💾 Committing proposal {} (Approved: {})", result.proposal_id, result.approved);
        
        // 1. Log result to in-memory storage
        self.local_env.storage.write().await.log_result(&result.proposal_id, result.clone());

        // 2. Persist to disk (simple audit file)
        let node_id = self.local_node.read().await.id.clone();
        let audit_dir = "audits";
        if let Err(e) = std::fs::create_dir_all(audit_dir) {
            warn!("Failed to create audit directory: {}", e);
        }
        let filename = format!("{}/audit-{}.json", audit_dir, node_id);
        self.local_env.export_audit(&filename).await;

        Ok(())
    }
}