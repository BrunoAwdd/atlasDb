use std::sync::Arc;

use tonic::{Request, Response, Status};

use crate::cluster_proto::{
    cluster_network_server::ClusterNetwork, 
    Ack, 
    HeartbeatMessage, 
    ProposalMessage, 
    VoteMessage
};

pub struct ClusterService {
    cluster: Arc<tokio::sync::RwLock<crate::cluster::cluster::Cluster>>,
}

impl ClusterService {
    pub fn new(cluster: Arc<tokio::sync::RwLock<crate::cluster::cluster::Cluster>>) -> Self {
        println!("Created");
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
        let ack = self.cluster.read().await.handle_heartbeat(request.into_inner()).await;
        Ok(Response::new(ack))
    }

    async fn submit_proposal(
        &self,
        request: Request<ProposalMessage>,
    ) -> Result<Response<Ack>, Status> {
        //let ack = self.cluster.read().await.handle_proposal(request.into_inner()).await;
        //Ok(Response::new(ack))
        Ok( Response::new(Ack { received: true, message: "Not implmented".to_string() })) // Placeholder response
    }

    async fn submit_vote(
        &self,
        request: Request<VoteMessage>,
    ) -> Result<Response<Ack>, Status> {
        //let ack = self.cluster.read().await.handle_vote(request.into_inner()).await;
        //Ok(Response::new(ack))
        Ok( Response::new(Ack { received: true, message: "Not implmented".to_string() }))
    }
}
