use std::sync::{Arc, RwLock};

use tokio::sync::oneshot;

use crate::{
    auth::Authenticator, 
    cluster::service::ClusterService, 
    cluster_proto::cluster_network_server::ClusterNetworkServer, 
    env::AtlasEnv, 
    network::adapter::NetworkAdapter, 
    Cluster, 
    NodeId
};

pub struct ClusterBuilder {
    env: Option<AtlasEnv>,
    network: Option<Arc<RwLock<dyn NetworkAdapter>>>,
    auth: Option<Arc<RwLock<dyn Authenticator>>>,
    node_id: Option<NodeId>,
}

impl ClusterBuilder {
    pub fn new() -> Self {
        Self {
            env: None,
            network: None,
            node_id: None,
            auth: None,
        }
    }

    pub fn with_env(mut self, env: AtlasEnv) -> Self {
        self.env = Some(env);
        self
    }

    pub fn with_network(mut self, network: Arc<RwLock<dyn NetworkAdapter>>) -> Self {
        self.network = Some(network);
        self
    }

    pub fn with_node_id(mut self, node_id: NodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }

    pub fn build(self) -> Result<Cluster, String> {
        let env = self.env.ok_or("Missing env")?;
        let network = self.network.ok_or("Missing network")?;
        let node_id = self.node_id.ok_or("Missing node_id")?;

        let cluster = Cluster::new(
            Arc::new(RwLock::new(env)), 
            Arc::clone(&network), 
            node_id
        );

        Ok(cluster)
    }

    /// Cria o cluster e já inicia o gRPC
    pub async fn start_with_grpc(self) -> Result<Arc<tokio::sync::RwLock<Cluster>>, Box<dyn std::error::Error>> {
        let cluster_build = self
            .build()
            .map_err(|e| format!("Build error: {}", e))?;

        let cluster = Arc::new(tokio::sync::RwLock::new(cluster_build));
        let grpc_cluster = cluster.clone(); // para uso no spawn

        let addr = {
            let cluster_read = grpc_cluster.read().await;
            cluster_read.local_node.address.parse()?
        };

        let service = ClusterService::new(grpc_cluster.clone());

        let (tx, rx) = oneshot::channel();

        {
            let mut guard = grpc_cluster.write().await;
            guard.shutdown_sender = Some(tx);
        }

        println!("🚀 Iniciando servidor gRPC em: {}", addr);

        // Aqui está a mágica: o servidor gRPC roda em segundo plano
        tokio::spawn(async move {
            if let Err(e) = tonic::transport::Server::builder()
                .add_service(ClusterNetworkServer::new(service))
                .serve_with_shutdown(addr, async {
                    rx.await.ok();
                })
                .await
            {
                eprintln!("Erro no servidor gRPC: {}", e);
            }
        });

        Ok(cluster)
    }
}

impl Default for ClusterBuilder {
    fn default() -> Self {
        Self::new()
    }
}