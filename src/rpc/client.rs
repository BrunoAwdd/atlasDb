use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
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

    let server_ca_cert = tokio::fs::read("certs/ca.pem").await?;
    let server_ca_cert = Certificate::from_pem(server_ca_cert);

    let client_cert = tokio::fs::read("certs/client.pem").await?;
    let client_key = tokio::fs::read("certs/client.key").await?;
    let client_identity = Identity::from_pem(client_cert, client_key);

    let tls_config = ClientTlsConfig::new()
        .domain_name("localhost")
        .ca_certificate(server_ca_cert)
        .identity(client_identity);

    for addr in node_addresses {
        let channel = match Channel::from_shared(addr.clone())?
            .tls_config(tls_config.clone())?
            .connect()
            .await
        {
            Ok(channel) => channel,
            Err(e) => {
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