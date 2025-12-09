use tonic::transport::Channel;
use crate::rpc::atlas::proposal_service_client::ProposalServiceClient;
use crate::rpc::atlas::{ProposalRequest, ProposalReply};

pub mod atlas {
    tonic::include_proto!("atlas");
}

pub async fn submit_proposal(
    node_addresses: Vec<String>,
    content: String,
) -> Result<ProposalReply, Box<dyn std::error::Error>> {
    let mut last_error = None;

    for addr in node_addresses {
        let channel = match Channel::from_shared(addr.clone())?
            .connect()
            .await
        {
            Ok(channel) => channel,
            Err(e) => {
                eprintln!("Connect error to {}: {:?}", addr, e);
                last_error = Some(Box::new(e) as Box<dyn std::error::Error>);
                continue;
            }
        };

        let mut client = ProposalServiceClient::new(channel);

        let request = tonic::Request::new(ProposalRequest {
            content: content.clone(),
        });

        match client.submit_proposal(request).await {
            Ok(response) => return Ok(response.into_inner()),
            Err(e) => {
                last_error = Some(Box::new(e) as Box<dyn std::error::Error>);
                continue;
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "No nodes available".into()))
}