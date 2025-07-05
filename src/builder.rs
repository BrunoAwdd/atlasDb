
use std::{
    net::{self, SocketAddr}, path, sync::{Arc, RwLock}
};
use env_logger::Env;
use tokio::sync::oneshot;
use tonic::transport::Server;

use crate::{
    cluster::{cluster::Cluster, node::Node, service::ClusterService}, env::{config::EnvConfig, consensus::ConsensusEngine, node::Graph, storage::Storage, AtlasEnv}, network::{
        adapter::NetworkAdapter,
        grcp_adapter::GRPCNetworkAdapter, 
    }, peer_manager::{self, PeerManager}, utils::NodeId
};



pub fn init(network: Arc<RwLock<dyn NetworkAdapter>>, path: Option<&str>) {
    let peer_manager = Arc::new(RwLock::new(PeerManager::new(10, 5)));
    create_env(network, peer_manager, path);
}

pub fn start(network: Arc<RwLock<dyn NetworkAdapter>>, path: Option<&str>) {
    let env = build_env(Arc::clone(&network), path);
    let node_id = NodeId("Node".to_string());
    Cluster::new(env, Arc::clone(&network), node_id);
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