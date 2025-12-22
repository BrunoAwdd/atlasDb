use crate::cluster::core::Cluster;
use atlas_common::{
    env::vote_data::{VoteData, vote_signing_bytes},
    error::{AtlasError, Result},
};

use atlas_common::{
    env::consensus::types::Vote,
};
use tracing::{info, warn};

impl Cluster {
    pub async fn create_vote(&self, proposal_id: &str, phase: atlas_common::env::consensus::types::ConsensusPhase) -> Result<Option<VoteData>> {
        // 1. Retrieve proposal
        let proposal = {
            let eng = self.local_env.engine.lock().await;
            eng.get_all_proposals().all().get(proposal_id).cloned()
        };

        let proposal = match proposal {
            Some(p) => p,
            None => return Ok(None),
        };

        // 2. Decide vote based on validity (and potentially previous phases)
        // For Prepare phase, we verify signature.
        // For PreCommit/Commit, we implicitly vote Yes if we reached this stage (quorum check done by caller).
        let vote = match phase {
            atlas_common::env::consensus::types::ConsensusPhase::Prepare => {
                let sign_bytes = atlas_common::env::proposal::signing_bytes(&proposal);
                let is_valid = self.auth.read().await
                    .verify_with_key(sign_bytes, &proposal.signature, &proposal.public_key)
                    .map_err(|e| AtlasError::Auth(format!("Verification failed: {}", e)))?;
                if is_valid { Vote::Yes } else { Vote::No }
            },
            _ => Vote::Yes, // If we are asked to vote PreCommit/Commit, it means we saw a quorum.
        };

        // 3. Create VoteData
        let mut vote_data = VoteData {
            proposal_id: proposal.id.clone(),
            vote: vote.clone(),
            voter: self.local_node.read().await.id.clone(),
            phase: phase.clone(),
            view: 0, // Default view for now
            signature: [0u8; 64],
            public_key: self.auth.read().await.public_key(),
        };

        // 4. Sign
        let msg = vote_signing_bytes(&vote_data);
        let sig_vec = self.auth.read().await.sign(msg)
            .map_err(|e| AtlasError::Auth(format!("Signing failed: {}", e)))?;
            
        let sig_arr: [u8; 64] = sig_vec
            .try_into()
            .map_err(|_| AtlasError::Auth("assinatura inválida: tamanho incorreto".to_string()))?;
        vote_data.signature = sig_arr;

        info!("📝 Publicando voto ({:?}): {:?}", phase, vote_data);
        tracing::info!(target: "consensus", "EVENT:VOTE proposal_id={} phase={:?} voter={} vote={:?}", vote_data.proposal_id, phase, vote_data.voter, vote_data.vote);

        Ok(Some(vote_data))
    }
        
    pub async fn handle_vote(&self, bytes: Vec<u8>) -> Result<Option<atlas_common::env::consensus::evidence::EquivocationEvidence>> {
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
                return Ok(None);
            }
        };
        drop(auth);

        let engine = self.local_env.engine.lock().await;
        // let votes = engine.get_all_votes().clone(); // removed verbose clone
        drop(engine); 

        tracing::info!(target: "consensus", "EVENT:RECEIVE_VOTE proposal_id={} voter={} vote={:?}", vote_data.proposal_id, vote_data.voter, vote_data.vote);

        if is_valid {
            self.local_env.storage.write().await.log_vote(
                &vote_data.proposal_id,
                vote_data.phase.clone(),
                vote_data.voter.clone(),
                vote_data.vote.clone()
            );

            // Return evidence if found by engine
            let evidence = self.local_env.engine.lock().await.receive_vote(vote_data.clone()).await;
            if evidence.is_some() {
                 return Ok(evidence);
            }
    
            Ok(None)
        } else {
            Ok(None)
        }
    }
}
