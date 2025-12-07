use std::sync::Arc;
use tonic::{Request, Response, Status};
use tonic::transport::Server;

use crate::runtime::maestro::Maestro;
use crate::network::p2p::ports::P2pPublisher;
use crate::rpc::atlas::{
    proposal_service_server::{ProposalService, ProposalServiceServer},
    ProposalRequest, ProposalReply,
};


// Define a struct para o nosso serviço. Ela precisa de acesso ao Maestro.
pub struct MyProposalService<P: P2pPublisher> {
    maestro: Arc<Maestro<P>>,
}

#[tonic::async_trait]
impl<P: P2pPublisher + 'static> ProposalService for MyProposalService<P> {
    // Implementa o método `submit_proposal` do nosso serviço gRPC.
    async fn submit_proposal(
        &self,
        request: Request<ProposalRequest>,
    ) -> Result<Response<ProposalReply>, Status> {
        println!("gRPC: Recebida chamada para SubmitProposal");

        let req = request.into_inner();

        // Aqui, chamamos a lógica de negócio que já existe no Maestro.
        match self.maestro.submit_external_proposal(req.content).await {
            Ok(proposal_id) => {
                let reply = ProposalReply {
                    message: "Proposta submetida com sucesso".into(),
                    proposal_id,
                };
                Ok(Response::new(reply))
            }
            Err(e) => {
                Err(Status::internal(format!("Falha ao submeter proposta: {}", e)))
            }
        }
    }
}

// Função para iniciar o servidor gRPC com mTLS.
pub async fn run_server<P: P2pPublisher + 'static>(
    maestro: Arc<Maestro<P>>,
    addr: std::net::SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("[Plaintext] Servidor gRPC escutando em {}", addr);

    let service = MyProposalService {
        maestro,
    };

    Server::builder()
        .add_service(ProposalServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
