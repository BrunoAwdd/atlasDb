use std::{net::UdpSocket, sync::Arc};
use tokio::sync::{oneshot, Mutex, RwLock};

use crate::{
    auth::Authenticator,
    cluster::{
        builder::ClusterBuilder, 
        core::Cluster, 
        service::ClusterService
    }, 
    cluster_proto::cluster_network_server::ClusterNetworkServer, 
    config::Config, 
    env::{
        config::EnvConfig, 
        AtlasEnv
    }, 
    network::{
        adapter::NetworkAdapter, 
        grcp_adapter::GRPCNetworkAdapter
    }, 
    peer_manager::PeerManager, 
    utils::NodeId, 
    Graph, 
    Storage
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

pub fn get_local_ip() -> std::net::IpAddr {
    let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind");
    socket.connect("8.8.8.8:80").expect("Failed to connect");
    socket.local_addr().expect("Failed to get local address").ip()
}

pub async fn load_config(path: &str, auth: Arc<RwLock<dyn Authenticator>>) -> Result<Arc<Cluster>, Box<dyn std::error::Error>> {
    let config = Config::load_from_file(path).or_else(|_| Config::load_from_file("config.json"))?;

    let network = Arc::new(
        GRPCNetworkAdapter::new(config.address.clone(), config.port.clone()),
    );

    let cluster = config.build_cluster_env(network, auth);
    
    let arc = start_with_grpc(cluster).await?;

    Ok(arc)
}

/// Cria o cluster e já inicia o gRPC
pub async fn start_with_grpc(mut cluster: Cluster) -> Result<Arc<Cluster>, Box<dyn std::error::Error>> {
    let addr = cluster.local_node.address.clone().parse()?; // ✅ fora do lock

    let (tx, rx) = oneshot::channel();
    cluster.shutdown_sender = Mutex::new(Some(tx));

    let arc_cluster = Arc::new(cluster);

    let service = ClusterService::new(arc_cluster.clone());

    println!("Starting gRPC server at {}", addr);

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

    Ok(arc_cluster)
}
