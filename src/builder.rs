use std::sync::{Arc, RwLock};
use crate::{
    auth::Authenticator, 
    cluster::{
        builder::ClusterBuilder, 
        core::Cluster
    }, 
    env::{
        config::EnvConfig, 
        AtlasEnv
    }, 
    network::adapter::NetworkAdapter,
    peer_manager::PeerManager, utils::NodeId
};

pub fn init(network: Arc<RwLock<dyn NetworkAdapter>>, path: Option<&str>) {
    let peer_manager = Arc::new(RwLock::new(PeerManager::new(10, 5)));
    create_env(network, peer_manager, path);
}

pub async fn start(
    network: Arc<RwLock<dyn NetworkAdapter>>, 
    path: Option<&str>, 
    id: String,
    auth: Arc<RwLock<dyn Authenticator>>
) -> Result<Arc<tokio::sync::RwLock<Cluster>>, Box<dyn std::error::Error>> {
    let env = build_env(Arc::clone(&network), path);
    let node_id = NodeId(id);
    let cluster = ClusterBuilder::new()
        .with_env(env)
        .with_network(Arc::clone(&network))
        .with_node_id(node_id)
        .with_auth(auth)
        .start_with_grpc()
        .await
        .map_err(|e| e.to_string())?; 

    Ok(cluster)
}

fn build_env(network: Arc<RwLock<dyn NetworkAdapter>>, path: Option<&str>) -> AtlasEnv {
    load_env(path.unwrap_or("config.json"), Arc::clone(&network))
}

fn create_env(
    network: Arc<RwLock<dyn NetworkAdapter>>, 
    peer_manager: Arc<RwLock<PeerManager>>, 
    path: Option<&str>
) -> AtlasEnv {
    AtlasEnv::new(
        network,
        Arc::new(|_| {}),
        peer_manager,
        path,
    )
}

fn load_env(path: &str, network: Arc<RwLock<dyn NetworkAdapter>>) -> AtlasEnv {
    EnvConfig::load_from_file(path)
        .expect("Failed to load config file")
        .build_env(network)
}

