use std::{net::UdpSocket, sync::Arc};
use tokio::sync::RwLock;
use atlas_common::{
    auth::Authenticator,
    utils::NodeId
};

use atlas_consensus::{
    Cluster,
    QuorumPolicy,
    ClusterBuilder,
};
use crate::config::Config;
use atlas_consensus::env::runtime::AtlasEnv;
use atlas_p2p::PeerManager;
use atlas_common::env::node::Graph;
use atlas_ledger::storage::Storage;
use crate::env_config::EnvConfig;



// Touched for rebuild
pub fn init(path: Option<&str>, node_id: Option<String>, config: Option<Config>) {
    let peer_manager = PeerManager::new(10, 5);
    let ip = get_local_ip().to_string();
    let node_id_str = node_id.clone().unwrap_or_else(|| "default".to_string());
    let data_dir = format!("data/{}", node_id_str);

    let config = config.unwrap_or(Config {
        node_id: NodeId(node_id.unwrap_or("".to_string())),
        address: ip,
        port: 50052,
        quorum_policy: QuorumPolicy::default(),
        graph: Graph::new(),
        storage: Storage::new(&data_dir),
        peer_manager,
        data_dir: data_dir.clone(),
    });

    config.save_to_file(path.unwrap_or("config.json")).expect("Failed to save initial configuration");
}

pub async fn start(
    path: Option<&str>, 
    id: String,
    auth: Arc<RwLock<dyn Authenticator>>
) -> Result<Arc<Cluster>, Box<dyn std::error::Error>> {
    let env = build_env(path);
    let node_id = NodeId(id);
    let cluster = ClusterBuilder::new()
        .with_env(env)
        .with_node_id(node_id)
        .with_auth(auth)
        .build()?;

    Ok(Arc::new(cluster))
}

fn build_env(path: Option<&str>) -> AtlasEnv {
    load_env(path.unwrap_or("config.json"))
}

fn load_env(path: &str) -> AtlasEnv {
    EnvConfig::load_from_file(path)
        .expect("Failed to load config file")
        .build_env()
}

pub fn get_local_ip() -> std::net::IpAddr {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind");
    socket.connect("8.8.8.8:80").expect("Failed to connect");
    socket.local_addr().expect("Failed to get local address").ip()
}

pub async fn load_config(path: &str, auth: Arc<RwLock<dyn Authenticator>>) -> Result<Arc<Cluster>, Box<dyn std::error::Error>> {
    let config = Config::load_from_file(path).or_else(|_| Config::load_from_file("config.json"))?;

    let cluster = config.build_cluster_env(auth).await;

    Ok(Arc::new(cluster))
}

