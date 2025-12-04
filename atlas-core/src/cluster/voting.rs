use crate::{
    cluster::core::Cluster,
    env::vote_data::{VoteData, vote_signing_bytes},
    error::{AtlasError, Result},
};

use atlas_sdk::{
    env::consensus::types::Vote,
};
use tracing::{info, warn};

impl Cluster {
    pub(crate) async fn vote_proposals(&self) -> Result<Vec<VoteData>> {
        // pega proposals sem segurar o lock
        let proposal_pool = {
            let eng = self.local_env.engine.lock().await;
            eng.get_all_proposals().all().clone()
        };

        let mut out = Vec::new();

        for (_, proposal) in proposal_pool {
            // 1) decide o voto
            // Use standardized signing bytes for proposal verification
            let sign_bytes = crate::env::proposal::signing_bytes(&proposal);
            let is_valid = self.auth.read().await
                .verify_with_key(sign_bytes, &proposal.signature, &proposal.public_key)
                .map_err(|e| AtlasError::Auth(format!("Verification failed: {}", e)))?;

            let vote = match is_valid {
                true => Vote::Yes,
                false => Vote::No,
            };

            // 2) monta VoteData (sem assinatura)
            let mut vote_data = VoteData {
                proposal_id: proposal.id.clone(),
                vote,
                voter: self.local_node.read().await.id.clone(),
                signature: [0u8; 64],
                public_key: self.auth.read().await.public_key(),
            };

            // 3) assina canonicamente
            let msg = vote_signing_bytes(&vote_data);
            let sig_vec = self.auth.read().await.sign(msg)
                .map_err(|e| AtlasError::Auth(format!("Signing failed: {}", e)))?;
                
            let sig_arr: [u8; 64] = sig_vec
                .try_into()
                .map_err(|_| AtlasError::Auth("assinatura inválida: tamanho incorreto".to_string()))?;
            vote_data.signature = sig_arr;

            info!("📝 Publicando voto: {:?}", vote_data);
            tracing::info!(target: "consensus", "EVENT:VOTE proposal_id={} voter={} vote={:?}", vote_data.proposal_id, vote_data.voter, vote_data.vote);

            // 4) publica no tópico atlas/vote/v1
            out.push(vote_data);
        }

        Ok(out)
    }
        
    pub(crate) async fn handle_vote(&self, bytes: Vec<u8>) -> Result<()> {
        let vote_data: VoteData = bincode::deserialize(&bytes)
            .map_err(|e| AtlasError::Other(format!("decode vote: {e}")))?;

        let signature_array: [u8; 64] = vote_data.signature
            .as_slice()
            .try_into()
            .map_err(|_| AtlasError::Auth("Assinatura com tamanho inválido".to_string()))?;

        let auth = self.auth.read().await;

        // Use standardized signing bytes for vote verification
        let sign_bytes = vote_signing_bytes(&vote_data);
        let is_valid = match auth.verify_with_key(sign_bytes, &signature_array, &vote_data.public_key) {
            Ok(valid) => valid,
            Err(e) => {
                warn!("Erro ao verificar assinatura do voto: {}", e);
                return Ok(());
            }
        };
        drop(auth);

        let engine = self.local_env.engine.lock().await;
        let votes = engine.get_all_votes().clone(); // clona os dados para sair do guard
        drop(engine); // opcional: solta o lock antes de usar votes

        info!("Votes {} {:?}", self.local_node.read().await.id, &votes);
        tracing::info!(target: "consensus", "EVENT:RECEIVE_VOTE proposal_id={} voter={} vote={:?}", vote_data.proposal_id, vote_data.voter, vote_data.vote);


        if is_valid {
            self.local_env.engine.lock().await.receive_vote(vote_data.clone()).await;
    
            Ok(())
        } else {
            Ok(())
        }
    }
}
