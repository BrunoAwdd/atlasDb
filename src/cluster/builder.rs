use std::sync::{Arc, RwLock};

use crate::{env::AtlasEnv, network::adapter::NetworkAdapter, Cluster, NodeId};

pub struct ClusterBuilder {
    env: Option<AtlasEnv>,
    network: Option<Arc<RwLock<dyn NetworkAdapter>>>,
    node_id: Option<NodeId>,
}

impl ClusterBuilder {
    pub fn new() -> Self {
        Self {
            env: None,
            network: None,
            node_id: None,
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

        let cluster = Cluster::new(env, Arc::clone(&network), node_id);

        Ok(cluster)
    }

    /// Cria o cluster e já inicia o gRPC
    pub async fn start_with_grpc(self) -> Result<Cluster, Box<dyn std::error::Error>> {
        let cluster = self
            .build()
            .map_err(|e| format!("Build error: {}", e))?;
        // Clona para passar ao tokio::spawn
        let mut cluster_clone = cluster.clone();

        tokio::spawn(async move {
            if let Err(e) = cluster_clone.serve_grpc().await {
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