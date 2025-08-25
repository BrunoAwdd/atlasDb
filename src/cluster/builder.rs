use std::sync::Arc;

use tokio::sync::{oneshot, RwLock, Mutex};

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
    network: Option<Arc<dyn NetworkAdapter>>,
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

    pub fn with_network(mut self, network: Arc<dyn NetworkAdapter>) -> Self {
        self.network = Some(network);
        self
    }

    pub fn with_node_id(mut self, node_id: NodeId) -> Self {
        self.node_id = Some(node_id);
        self
    }

    pub fn with_auth(mut self, auth: Arc<RwLock<dyn Authenticator>>) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn build(self) -> Result<Cluster, String> {
        let env = self.env.ok_or("Missing env")?;
        let network = self.network.ok_or("Missing network")?;
        let node_id = self.node_id.ok_or("Missing node_id")?;
        let auth = self.auth.ok_or("Missing auth")?;

        let cluster = Cluster::new(
            env, 
            Arc::clone(&network), 
            node_id,
            auth
        );

        Ok(cluster)
    }

    /// Cria o cluster e já inicia o gRPC
    pub async fn start_with_grpc(self) -> Result<Arc<Cluster>, Box<dyn std::error::Error>> {
        let mut cluster_build = self
            .build()
            .map_err(|e| format!("Build error: {}", e))?;

        let (tx, rx) = oneshot::channel();

        cluster_build.shutdown_sender = Mutex::new(Some(tx));

        let cluster = Arc::new(cluster_build);
        let grpc_cluster = cluster.clone(); // para uso no spawn

        let addr = cluster.local_node.address.parse()
            .map_err(|e| format!("Invalid address: {}", e))?;

        let service = ClusterService::new(grpc_cluster.clone());


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