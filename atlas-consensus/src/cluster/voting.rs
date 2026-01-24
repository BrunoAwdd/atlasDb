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
        let (proposal, already_voted) = {
            let eng = self.local_env.engine.lock().await;
            let registry = eng.get_all_votes(); // We need access to check duplicates
            // Implementation detail: we need to check if we voted for THIS view/phase for ANY proposal.
            // But registry structure is `votes_by_view: HashMap<u64, HashMap<Phase, HashMap<NodeId, VoteData>>>`
            // We need to inspect this.
            // Since `registry` field is private/internal specific, we might rely on a helper or check logically.
            // Accessing `votes_by_view` is direct if we are inside consensus crate or have pub access.
            // In `engine.rs` we have `get_all_votes()` returning `&VoteRegistry`.
            // In `registry.rs`, `votes_by_view` is private?
            // Let's assume we can add a helper `has_voted(view, phase, voter) -> bool` to VoteRegistry or just check blindly.
            
            // To be safe and quick without changing Registry api deeply:
            // Just check if there is a conflict.
            // Wait, we can't access `votes_by_view` if it's private.
            // We'll trust the Engine or add a check in `ConsensusEvaluator`? No.
            
            // Let's try to fetch proposal first.
            let p = eng.get_all_proposals().all().get(proposal_id).cloned();
            (p, false) // Placeholder for vote check, see below
        };
        
        let proposal = match proposal {
            Some(p) => p,
            None => return Ok(None),
        };

        // SAFETY CHECK: Prevent Double Voting (Self-Equivocation)
        // We must check if we already voted for this View & Phase.
        {
            let eng = self.local_env.engine.lock().await;
            let reg = eng.get_all_votes();
            let my_id = self.local_node.read().await.id.clone();
            
            let voted = reg.has_voted(0, &phase, &my_id);
            if voted {
                 if let Some(existing) = reg.get_vote_by_view(0, &phase, &my_id) {
                     if existing.proposal_id != proposal_id {
                         warn!("🛑 Prevented Self-Equivocation! Already voted for {} in View 0/{:?}, refusing to vote for {}.", existing.proposal_id, phase, proposal_id);
                         return Ok(None);
                     } else {
                         info!("🔄 Idempotent retry: Already voted for this proposal. Re-broadcasting/Re-returning.");
                     }
                 }
            } else {
                info!("🟢 No previous vote found for View 0/{:?}. Proceeding to vote on {}.", phase, proposal_id);
            }
        }

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

        // 5. ATOMIC PRE-REGISTRATION
        // Register the vote locally BEFORE broadcasting. 
        // This catches any self-equivocation (checking against previous votes) 
        // and "locks" our vote for this view/phase.
        {
            let mut eng = self.local_env.engine.lock().await;
            match eng.registry.register_vote(vote_data.clone()) {
                Ok(Some(_evidence)) => {
                     warn!("🛑 ATOMIC GUARD: Attempted to vote for proposal {} but already voted for another in View {}/{:?}. Vote aborted.", vote_data.proposal_id, vote_data.view, vote_data.phase);
                     return Ok(None);
                },
                Err(e) => {
                    warn!("Failed to register vote locally: {}", e);
                    return Ok(None);
                },
                Ok(None) => {
                    // Vote accepted locally. Proceed to broadcast.
                }
            }
        }

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
