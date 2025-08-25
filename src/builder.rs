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

pub fn init(path: Option<&str>, node_id: Option<String>, config: Option<Config>) {
    let peer_manager = PeerManager::new(10, 5);
    let ip = get_local_ip().to_string();

    let config = config.unwrap_or(Config {
        node_id: NodeId(node_id.unwrap_or("".to_string())),
        address: ip,
        port: 50052,
        quorum_ratio: 0.5,
        graph: Graph::new(),
        storage: Storage::new(),
        peer_manager,
    });

    config.save_to_file(path.unwrap_or("config.json")).expect("Failed to save initial configuration");
}

pub async fn start(
    network: Arc<dyn NetworkAdapter>, 
    path: Option<&str>, 
    id: String,
    auth: Arc<RwLock<dyn Authenticator>>
) -> Result<Arc<Cluster>, Box<dyn std::error::Error>> {
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

fn build_env(network: Arc<dyn NetworkAdapter>, path: Option<&str>) -> AtlasEnv {
    load_env(path.unwrap_or("config.json"), Arc::clone(&network))
}

fn load_env(path: &str, network: Arc<dyn NetworkAdapter>) -> AtlasEnv {
    EnvConfig::load_from_file(path)
        .expect("Failed to load config file")
        .build_env(network)
}

