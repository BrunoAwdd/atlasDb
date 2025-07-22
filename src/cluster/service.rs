use std::{
    sync::Arc, 
    time::Duration
};

use tokio::time::timeout;
use tonic::{Request, Response, Status};

use crate::cluster_proto::{
    cluster_network_server::ClusterNetwork, 
    Ack, 
    HeartbeatMessage, 
    ProposalMessage, 
    VoteMessage,
    VoteBatch,
    ProposalBatch
};

pub struct ClusterService {
    cluster: Arc<tokio::sync::RwLock<crate::cluster::core::Cluster>>,
}

impl ClusterService {
    pub fn new(
        cluster: Arc<tokio::sync::RwLock<crate::cluster::core::Cluster>>
    ) -> Self {
        ClusterService { cluster }
    }
}

#[tonic::async_trait]
impl ClusterNetwork for ClusterService {
    async fn heartbeat(
        &self,
        request: Request<HeartbeatMessage>,
    ) -> Result<Response<Ack>, Status> {
        println!("Received heartbeat from: {}", request.get_ref().from);
        tokio::time::sleep(tokio::time::Duration::from_secs(12)).await;

        let ack = self
            .cluster
            .read()
            .await
            .handle_heartbeat(request.into_inner());
        Ok(Response::new(ack))
    }

    async fn submit_vote_batch(
        &self,
        request: Request<VoteBatch>,
    ) -> Result<Response<Ack>, Status> {
        println!("Received vote batch from: {}", request.get_ref().votes[0].voter_id);

        let mut cluster = self.cluster.write().await;

        let ack = cluster
            .handle_vote_batch(request.into_inner())
            .map_err(|e| Status::internal(format!("handle_vote_batch error: {}", e)))?;

        Ok(Response::new(ack))
    }

    async fn submit_proposal_batch(
        &self,
        request: Request<ProposalBatch>,
    ) -> Result<Response<Ack>, Status> {
        let proposals = &request.get_ref().proposals;
    
        if let Some(first) = proposals.get(0) {
            println!("Received proposal batch from (service): {}", first.proposer_id);
        } else {
            println!("Received empty proposal batch.");
            return Err(Status::invalid_argument("Empty proposal batch"));
        }
    
        let prop = request.into_inner();
    
        println!("🟢 Tentando adquirir lock de escrita no cluster...");
    
        let write_result = timeout(Duration::from_secs(50), self.cluster.write()).await;
    
        match write_result {
            Ok(mut cluster) => {
                println!("🟡 Lock adquirido com sucesso!");
                let ack = cluster
                    .handle_proposal_batch(prop)
                    .map_err(|e| {
                        eprintln!("❌ handle_proposal_batch error: {}", e);
                        Status::internal(format!("handle_proposal_batch error: {}", e))
                    })?;
    
                Ok(Response::new(ack))
            }
            Err(_) => {
                eprintln!("❌ Timeout ao tentar adquirir lock — possível deadlock no cluster");
                Err(Status::internal("Timeout ao tentar acessar cluster — possível deadlock"))
            }
        }
    }

    async fn submit_proposal(
        &self,
        request: Request<ProposalMessage>,
    ) -> Result<Response<Ack>, Status> {
        println!("Received proposal from: {}", request.get_ref().proposer_id);
        let ack = self.cluster.write().await.handle_proposal(request.into_inner()).map_err(|e| Status::internal(format!("handle_proposal error: {}", e)))?;
        Ok(Response::new(ack))
    }

    async fn submit_vote(
        &self,
        request: Request<VoteMessage>,
    ) -> Result<Response<Ack>, Status> {
        // TODO: Implement this
        //let ack = self.cluster.read().await.handle_vote(request.into_inner()).await;
        //Ok(Response::new(ack))
        Ok( Response::new(Ack { received: true, message: "Not implemented".to_string() }))
    }
}
