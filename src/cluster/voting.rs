use crate::{
    cluster::core::Cluster,
    env::vote_data::{VoteData, vote_signing_bytes},
    Vote,
};

impl Cluster {
    pub(crate) async fn vote_proposals(&self) -> Result<Vec<VoteData>, String> {
        // pega proposals sem segurar o lock
        let proposal_pool = {
            let eng = self.local_env.engine.lock().await;
            eng.get_all_proposals().all().clone()
        };

        let mut out = Vec::new();

        for (_, proposal) in proposal_pool {
            // 1) decide o voto
            let serialized = bincode::serialize(&proposal.content).unwrap();
            let is_valid = self.auth.read().await.verify(serialized, &proposal.signature);
            let vote = match is_valid {
                Ok(true) => Vote::Yes,
                Ok(false) => Vote::No,
                Err(_) => Vote::Abstain,
            };

            // 2) monta VoteData (sem assinatura)
            let mut vote_data = VoteData {
                proposal_id: proposal.id.clone(),
                vote,
                voter: self.local_node.id.clone(),
                signature: [0u8; 64],
                public_key: vec![],
            };

            // 3) assina canonicamente
            let msg = vote_signing_bytes(&vote_data);
            let sig_vec = self.auth.read().await.sign(msg, "12345".to_string())?;
            let sig_arr: [u8; 64] = sig_vec
                .try_into()
                .map_err(|_| "assinatura inválida: tamanho incorreto")?;
            vote_data.signature = sig_arr;
            // se seu Auth expõe pubkey:
            // vote_data.public_key = self.auth.read().await.public_key_bytes()?;

            println!("📝 Publicando voto: {:?}", vote_data);

            // 4) publica no tópico atlas/vote/v1
            out.push(vote_data);
        }

        Ok(out)
    }
        
    pub(crate) async fn handle_vote(&self, bytes: Vec<u8>) -> Result<(), String> {
        let vote_data: VoteData = bincode::deserialize(&bytes)
            .map_err(|e| format!("decode vote: {e}"))?;

        let signature_array: [u8; 64] = vote_data.signature
            .as_slice()
            .try_into()
            .map_err(|_| "Assinatura com tamanho inválido")?;

        let auth = self.auth.read().await;

        let is_valid = match auth.verify(bytes, &signature_array) {
            Ok(valid) => valid,
            Err(e) => {
                return Ok(());
            }
        };
        drop(auth);

        let engine = self.local_env.engine.lock().await;
        let votes = engine.get_all_votes().clone(); // clona os dados para sair do guard
        drop(engine); // opcional: solta o lock antes de usar votes

        println!("Votes {} {:?}", self.local_node.id, &votes);


        if is_valid {
            self.local_env.engine.lock().await.receive_vote(vote_data.clone()).await;
    
            Ok(())
        } else {
            Ok(())
        }
    }
}
