pub mod cluster;
//pub mod ffi;
pub mod network;
pub mod utils;
pub mod peer_manager;
pub mod env;
pub mod mock;

pub mod cluster_proto {
    tonic::include_proto!("cluster");
}

use std::{
    sync::{Arc, RwLock},
    net::SocketAddr
};
use tokio::sync::oneshot;
use tonic::transport::Server;
use cluster_proto::cluster_network_server::ClusterNetworkServer;

use crate::{
    env::{config::EnvConfig, AtlasEnv},
    cluster::{cluster::Cluster, node::Node, service::ClusterService}, env::{
        consensus::ConsensusEngine,
        node::Graph,
        storage::Storage, 
        
    }, 
    network::{
        adapter::NetworkAdapter,
        grcp_adapter::GRPCNetworkAdapter, 
    }, 

    peer_manager::PeerManager, 
    utils::NodeId
};
pub mod builder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>  {   
        let a_adapter = Arc::new(RwLock::new(
            GRPCNetworkAdapter::new( "[::1]".to_string(), 50052)
        ));
        
        let b_adapter = Arc::new(RwLock::new(
            GRPCNetworkAdapter::new("[::1]".to_string(), 50051)
        ));
        let env_a_conf = EnvConfig::load_from_file("env_a.json").expect("Failed to load config file");
        let env_a = env_a_conf.build_env(a_adapter.clone());

        let env_b_conf = EnvConfig::load_from_file("env_b.json").expect("Failed to load config file");
        let env_b = env_b_conf.build_env(a_adapter.clone());
    
        let mut cluster_a = Cluster::new(env_a.clone(), a_adapter.clone(), NodeId("NodeA".into()));
        let mut cluster_b = Cluster::new(env_b.clone(), b_adapter.clone(), NodeId("NodeB".into()));
    
        cluster_a.add_node(cluster_b.local_node.id.clone(), cluster_b.local_node.clone());
        cluster_b.add_node(cluster_a.local_node.id.clone(), cluster_a.local_node.clone());
    
    
        // Start GRPC Servers
        let (tx1, rx1) = oneshot::channel();
        let (tx2, rx2) = oneshot::channel();
    
        let addr1: SocketAddr = cluster_a.local_node.address.parse()?;
        let addr2: SocketAddr = cluster_b.local_node.address.parse()?;
    
        let node1 = ClusterService::new(Arc::new(tokio::sync::RwLock::new(cluster_a.clone())));
        let node2 = ClusterService::new(Arc::new(tokio::sync::RwLock::new(cluster_b.clone())));
    
        println!("Node A: {:?}", cluster_a.local_node);
        println!("Node B: {:?}", cluster_b.local_node);
    
    
        
        // Servidor 1
        let srv1 = tokio::spawn(async move {
            Server::builder()
                .add_service(ClusterNetworkServer::new(node1))
                .serve_with_shutdown(addr1, async {
                    rx1.await.ok();
                })
                .await
                .unwrap();
        });
    
        // Servidor 2
        let srv2 = tokio::spawn(async move {
            Server::builder()
                .add_service(ClusterNetworkServer::new(node2))
                .serve_with_shutdown(addr2, async {
                    rx2.await.ok();
                })
                .await
                .unwrap();
        });
    
        // Dá um tempo para os servidores subirem
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        cluster_a.send_heartbeat(NodeId("NodeB".into()), "any".to_string()).await;
    
        // Encerrar os servidores
        tx1.send(()).unwrap();
        tx2.send(()).unwrap();
        srv1.await?;
        srv2.await?;
        
        env_a.save_config("env_a.json").expect("Failed to save config A");
        env_b.save_config("env_b.json").expect("Failed to save config B");
    
        Ok(())
}


#[tokio::main]
async fn maina() -> Result<(), Box<dyn std::error::Error>>  {
    // Start Cluster
    let peer_manager_a = Arc::new(RwLock::new(PeerManager::new(10, 5)));
    let peer_manager_b = Arc::new(RwLock::new(PeerManager::new(20, 10)));

    let a_adapter = Arc::new(RwLock::new(
        GRPCNetworkAdapter::new( "[::1]".to_string(), 50052)
    ));
    
    let b_adapter = Arc::new(RwLock::new(
        GRPCNetworkAdapter::new("[::1]".to_string(), 50051)
    ));

    let env_a = create_env(a_adapter.clone(), peer_manager_a.clone(), Some("env_a.json"));
    let env_b = create_env(b_adapter.clone(), peer_manager_b.clone(), Some("env_b.json"));

    let env_conf = EnvConfig::load_from_file("env_a.json").expect("Failed to load config file");
    let env_c = env_conf.build_env(a_adapter.clone());

    println!("📝 Configuração carregada C: {:?}", env_c.peer_manager);
    println!("📝 Configuração carregada B: {:?}", env_b.peer_manager);
    println!("📝 Configuração carregada A: {:?}", env_a.peer_manager);

    let mut cluster_a = Cluster::new(env_a, a_adapter.clone(), NodeId("NodeA".into()));
    let mut cluster_b = Cluster::new(env_b, b_adapter.clone(), NodeId("NodeB".into()));

    cluster_a.add_node(cluster_b.local_node.id.clone(), cluster_b.local_node.clone());
    cluster_b.add_node(cluster_a.local_node.id.clone(), cluster_a.local_node.clone());


    // Start GRPC Servers
    let (tx1, rx1) = oneshot::channel();
    let (tx2, rx2) = oneshot::channel();

    let addr1: SocketAddr = cluster_a.local_node.address.parse()?;
    let addr2: SocketAddr = cluster_b.local_node.address.parse()?;

    let node1 = ClusterService::new(Arc::new(tokio::sync::RwLock::new(cluster_a.clone())));
    let node2 = ClusterService::new(Arc::new(tokio::sync::RwLock::new(cluster_b.clone())));

    println!("Node A: {:?}", cluster_a.local_node);
    println!("Node B: {:?}", cluster_b.local_node);


    
    // Servidor 1
    let srv1 = tokio::spawn(async move {
        Server::builder()
            .add_service(ClusterNetworkServer::new(node1))
            .serve_with_shutdown(addr1, async {
                rx1.await.ok();
            })
            .await
            .unwrap();
    });

    // Servidor 2
    let srv2 = tokio::spawn(async move {
        Server::builder()
            .add_service(ClusterNetworkServer::new(node2))
            .serve_with_shutdown(addr2, async {
                rx2.await.ok();
            })
            .await
            .unwrap();
    });

        // Dá um tempo para os servidores subirem
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        cluster_a.send_heartbeat(NodeId("NodeB".into()), "any".to_string()).await;
    
        // Encerrar os servidores
        tx1.send(()).unwrap();
        tx2.send(()).unwrap();
        srv1.await?;
        srv2.await?;
    

        Ok(())
}



fn create_env(network: Arc<RwLock<dyn NetworkAdapter>>, peer_manager: Arc<RwLock<PeerManager>>, path: Option<&str>) -> AtlasEnv {
   AtlasEnv::new(
        network,
        Arc::new(|_| {}),
        peer_manager,
        path,
    )
}

