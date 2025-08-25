use std::{
    sync::Arc, 
};

use serde_json::Value;
use tonic::{Request, Response, Status};

use crate::{
    cluster_proto::{
        cluster_network_server::ClusterNetwork, 
        Ack, 
        HeartbeatMessage,
        ProposalMessage, 
        VoteMessage,
    },
    cluster::command::ClusterCommand,
};

pub struct ClusterService {
    cluster: Arc<crate::cluster::core::Cluster>,
}

impl ClusterService {
    pub fn new(
        cluster: Arc<crate::cluster::core::Cluster>
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
        let msg = request.into_inner();
        let cluster = self.cluster.clone();

        //println!("Received heartbeat from: {}", msg.from.clone());

        let handle = ClusterCommand::HandleHeartbeat(msg);
        let _ = handle.execute(&cluster).await;
        
        Ok(Response::new(Ack {
            received: true,
            message: "ACK em processamento...".to_string(),
        }))
    }

    async fn submit_vote(
        &self,
        request: Request<VoteMessage>,
    ) -> Result<Response<Ack>, Status> {
        println!("Received vote batch from: {}", request.get_ref().voter_id);


        let ack = self
            .cluster
            .handle_vote(request.into_inner())
            .await
            .map_err(|e| Status::internal(format!("handle_vote_batch error: {}", e)))?;

        Ok(Response::new(ack))
    }

    async fn submit_proposal(
        &self,
        request: Request<ProposalMessage>,
    ) -> Result<Response<Ack>, Status> {
        println!("Received proposal batch from: {}", request.get_ref().proposer_id);

        let raw = &request.get_ref().content;

        let parsed: Value = serde_json::from_str(raw).map_err(|e| {
            Status::invalid_argument(format!("Invalid JSON in proposal content: {}", e))
        })?;

        if let Some(array) = parsed.as_array() {
            if let Some(_) = array.get(0) {
                println!(
                    "Received proposal batch from (service): {}",
                    request.get_ref().proposer_id
                );
            } else {
                println!("Received empty proposal batch.");
                return Err(Status::invalid_argument("Empty proposal batch"));
            }
        } else {
            println!("Proposal content is not an array.");
            return Err(Status::invalid_argument("Expected array of proposals"));
        }


        let prop = request.into_inner();
    
        println!("🟢 Tentando adquirir lock de escrita no cluster {:?}...", self.cluster.local_node.id);
    
        let ack = self
            .cluster
            .handle_proposal(prop)
            .await
            .map_err(|e| {
                eprintln!("❌ handle_proposal_batch error: {}", e);
                Status::internal(format!("handle_proposal_batch error: {}", e))
            })?;

        Ok(Response::new(ack))
    }
}
