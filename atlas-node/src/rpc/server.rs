use std::sync::Arc;
use tonic::{Request, Response, Status};
use tonic::transport::Server;

use crate::runtime::maestro::Maestro;
use atlas_p2p::ports::P2pPublisher;
use crate::rpc::atlas::{
    proposal_service_server::{ProposalService, ProposalServiceServer},
    ProposalRequest, ProposalReply, StatusRequest, StatusReply,
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

    async fn get_status(
        &self,
        _request: Request<StatusRequest>,
    ) -> Result<Response<StatusReply>, Status> {
        let (node_id, leader_id, height, view) = self.maestro.get_status().await;
        
        let reply = StatusReply {
            node_id,
            leader_id,
            height,
            view,
        };

        Ok(Response::new(reply))
    }
}

// Função para iniciar o servidor gRPC com mTLS.
pub async fn run_server<P: P2pPublisher + 'static>(
    maestro: Arc<Maestro<P>>,
    ledger: Arc<atlas_ledger::Ledger>,
    mempool: Arc<atlas_mempool::Mempool>,
    addr: std::net::SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("[Unified] Servidor gRPC escutando em {}", addr);

    let proposal_service = MyProposalService {
        maestro,
    };

    let ledger_service = atlas_ledger::interface::api::service::LedgerServiceImpl {
        ledger,
        mempool,
    };

    use atlas_ledger::interface::api::service::ledger_proto::ledger_service_server::LedgerServiceServer;

    // CORS configuration for browser access
    let cors = tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::any())
        .allow_headers(tower_http::cors::AllowHeaders::any())
        .allow_methods(tower_http::cors::AllowMethods::any());

    Server::builder()
        .accept_http1(true)
        .layer(cors)
        .add_service(tonic_web::enable(ProposalServiceServer::new(proposal_service)))
        .add_service(tonic_web::enable(LedgerServiceServer::new(ledger_service)))
        .serve(addr)
        .await?;

    Ok(())
}
