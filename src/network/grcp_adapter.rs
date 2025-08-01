use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    cluster_proto::{
        cluster_network_client::ClusterNetworkClient,
        HeartbeatMessage,
        VoteMessage,
    },
    network::{
        adapter::ClusterMessage,
        error::NetworkError,
    },
    utils::NodeId,
    Node,
    Proposal,
};

use super::adapter::NetworkAdapter;

#[derive(Clone, Debug)]
pub struct GRPCNetworkAdapter {
    pub rcp_ip: String,
    pub rcp_port: u16,
}

impl GRPCNetworkAdapter {
    pub fn new( rcp_ip: String, rcp_port: u16) -> Self {
        GRPCNetworkAdapter { rcp_ip, rcp_port }
    }
}

#[async_trait::async_trait]
impl NetworkAdapter for GRPCNetworkAdapter {
    fn get_address(&self) -> String {
        format!("{}:{}", self.rcp_ip, self.rcp_port)
    }

    async fn broadcast(&self, msg: ClusterMessage) -> Result<(), NetworkError> {
        // Implement the broadcast logic using gRPC
        let _message = format!("Broadcast message: {:?}", msg);
        Ok(())
    }

    async fn send_votes(&self, target: Node, msg: ClusterMessage) -> Result<(), NetworkError> {
        println!("📡 Enviando votos para [{}] via gRPC", target.id);
        
        let addr = format!("http://{}", target.address);
        
        let mut client = ClusterNetworkClient::connect(addr)
            .await
            .map_err(|e| NetworkError::ConnectionError(e.to_string()))?;

        match &msg {
            ClusterMessage::Vote { proposal_id, vote, voter, public_key, signature } => {
                let vote_message = VoteMessage {
                    proposal_id: proposal_id.clone(),
                    voter_id: (*voter).to_string(),
                    vote: vote.clone().into(),
                    signature: signature.clone(),
                    public_key: public_key.clone(),
                };


                client
                    .submit_vote(tonic::Request::new(vote_message))
                    .await
                    .map(|r| {
                        println!("✅ ACK de {}: {:?}", target.id, r.into_inner());
                    })
                    .map_err(|e| NetworkError::Send(e.to_string()))?;
            }
            _ => {
                return Err(NetworkError::InvalidMessage);
            }
        }
        
        Ok(())
    }

    async fn send_proposal_batch(&self, target: Node, proposals_batch: ProposalBatch) -> Result<(), NetworkError> {
        println!("📡 Enviando propostas para [{}] via gRPC", target.id);
        
        let addr = format!("http://{}", target.address);
        
        let mut client = ClusterNetworkClient::connect(addr)
            .await
            .map_err(|e| NetworkError::ConnectionError(e.to_string()))?;
        
        client
            .submit_proposal_batch(tonic::Request::new(proposals_batch))
            .await
            .map_err(|e| NetworkError::Send(e.to_string()))?;
        
        Ok(())
    }

    async fn send_to(&self, target: Node, msg: ClusterMessage) -> Result<ClusterMessage, NetworkError> {
        println!("📡 Enviando mensagem para [{}] via gRPC", target.id);
    
        let addr = format!("http://{}", target.address);
    
        let mut client = ClusterNetworkClient::connect(addr)
            .await
            .map_err(|e| NetworkError::ConnectionError(e.to_string()))?;
    
        match &msg {
            ClusterMessage::Proposal { proposal, .. } => {
                client
                    .submit_proposal(tonic::Request::new(proposal.clone().into_proto()))
                    .await
                    .map(|r| {
                        println!("✅ ACK de {}: {:?}", target.id, r.into_inner());
                    })
                    .map_err(|e| NetworkError::Send(e.to_string()))?;
            }
            _ => {
                return Err(NetworkError::InvalidMessage);
            }
        }
    
        Ok(msg)
    }

    fn set_message_handler(&mut self, handler: Arc<dyn Fn(ClusterMessage) + Send + Sync>) {
        let _handler = handler;
        // TODO: Implement this
    }

    async fn send_heartbeat(&self, sender: NodeId, receiver: Node) -> Result<ClusterMessage, NetworkError> {
        let addr = format!("http://{}", receiver.address);

        println!("⏱️ Enviando heartbeat para [{}], ip[{}] em [{}] (GRCP)", receiver.id, addr, SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_e| NetworkError::Unknown)?.as_secs());


        let mut client =  ClusterNetworkClient::connect(addr).await.map_err(|e| NetworkError::ConnectionError(e.to_string()))?;
    
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_e| NetworkError::Unknown)?.as_secs() as u64;

        let msg = HeartbeatMessage {
            from: sender.0.clone(),
            timestamp
        };
    
        client.heartbeat(tonic::Request::new(msg))
            .await
            .map(|r| {
                println!("✅ ACK de {}: {:?}", receiver.id, r.into_inner());
            })
            .map_err(|e| NetworkError::Send(e.to_string()))?;

        let cluster_msg = ClusterMessage::Heartbeat {
            sender: sender.clone(),
            receiver: receiver.id.clone(),
            from: sender.clone(),
            timestamp,
        };

        Ok(cluster_msg)
    }
}