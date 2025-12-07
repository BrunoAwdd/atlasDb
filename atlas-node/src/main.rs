use atlas_common::{
    auth::{
        ed25519::Ed25519Authenticator,
        Authenticator
    },
    utils::NodeId
};

use atlas_p2p::config::P2pConfig;

use atlas_node::{
    config::Config,
    runtime::builder::build_runtime,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use clap::Parser;
use tracing::{info, error};
use env_logger::Env;
use std::path::Path;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Inicializar o logger
    let args: Vec<String> = std::env::args().collect();
    
    // Check if running in CLI mode (no args or specific flag)
    // Wait, the original main.rs had logic to run CLI if no args.
    if args.len() <= 1 {
        if let Err(e) = atlas_node::runtime::builder::run_cli().await {
            eprintln!("Error: {}", e);
        }
        return Ok(());
    }

    let p2p_listen_addr = get_arg_value(&args, "--listen").unwrap_or("/ip4/0.0.0.0/tcp/0");
    let dial_addr = get_arg_value(&args, "--dial");
    let grpc_port = get_arg_value(&args, "--grpc-port").unwrap_or("50051");
    let config_path = get_arg_value(&args, "--config").unwrap_or("config.json");
    let keypair_path = get_arg_value(&args, "--keypair").unwrap_or("keys/keypair");

    // Extract node name from config path (e.g., "node1/config.json" -> "node1")
    let node_name = std::path::Path::new(config_path)
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("unknown_node");

    let log_filename = format!("logs/consensus-{}.log", node_name);

    let file_appender = tracing_appender::rolling::never(".", log_filename);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let consensus_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
            metadata.target() == "consensus"
        }));

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_filter(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,atlas_node=debug".into())); // Changed atlas_db to atlas_node

    tracing_subscriber::registry()
        .with(consensus_layer)
        .with(stdout_layer)
        .init();

    info!("--- INICIANDO NÓ ATLASDB ---");
    info!("Config: {}", config_path);
    info!("Endereço P2P: {}", p2p_listen_addr);
    if let Some(addr) = dial_addr { info!("Bootstrap (dial): {}", addr); }
    info!("Porta gRPC: {}", grpc_port);

    // 3. Configuração do nó
    use atlas_p2p::key_manager; 
    
    let keypair = key_manager::load_or_generate_keypair(Path::new(keypair_path))?;
    let auth = Arc::new(RwLock::new(convert_libp2p_keypair(keypair.clone())?));

    // Load config to get bootstrap peers
    let config = Config::load_from_file(config_path)?;
    let mut bootstrap_peers = Vec::new();
    
    // Add CLI dial addr if present
    if let Some(addr) = dial_addr {
        bootstrap_peers.push(addr.to_string());
    }

    let p2p_config = P2pConfig {
        listen_multiaddrs: vec![p2p_listen_addr.into()],
        bootstrap: bootstrap_peers,
        enable_mdns: true,
        enable_kademlia: true,
        keypair_path: keypair_path.to_string(),
    };

    let grpc_addr_str = format!("0.0.0.0:{}", grpc_port);
    let grpc_addr = grpc_addr_str.parse()?;

    // 4. Construir e iniciar o runtime
    match build_runtime(config_path, auth, p2p_config, grpc_addr).await {
        Ok(_runtime) => {
            info!("Nó iniciado com sucesso. Pressione Ctrl+C para parar.");
        }
        Err(e) => {
            error!("Falha ao iniciar o nó: {}.", e);
            return Err(e.into());
        }
    }

    // 5. Manter o processo principal vivo
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
}

fn get_arg_value<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|pos| args.get(pos + 1))
        .map(|s| s.as_str())
}

fn convert_libp2p_keypair(keypair: libp2p::identity::Keypair) -> Result<Ed25519Authenticator, Box<dyn std::error::Error>> {
    let ed25519_keypair = keypair.try_into_ed25519()
        .map_err(|_| "Keypair is not Ed25519")?;
    
    // Extract secret key bytes. 
    let secret = ed25519_keypair.secret();
    let secret_bytes = secret.as_ref();
    
    // ed25519-dalek SigningKey::from_bytes takes 32 bytes (seed).
    Ed25519Authenticator::from_bytes(secret_bytes).map_err(|e| e.into())
}
