use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::Path;
use atlas_sdk::auth::ed25519::Ed25519Authenticator;
use atlas_db::network::key_manager;
use tracing::{info, error};

use atlas_db::network::p2p::config::P2pConfig;
use atlas_db::runtime::builder::build_runtime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Inicializar o logger
    tracing_subscriber::fmt()
        .with_env_filter("info,atlas_db=debug")
        .init();

    // 2. Parsear argumentos da linha de comando
    let args: Vec<String> = std::env::args().collect();
    let p2p_listen_addr = get_arg_value(&args, "--listen").unwrap_or("/ip4/0.0.0.0/tcp/0");
    let dial_addr = get_arg_value(&args, "--dial");
    let grpc_port = get_arg_value(&args, "--grpc-port").unwrap_or("50051");
    let config_path = get_arg_value(&args, "--config").unwrap_or("config.json");
    let keypair_path = get_arg_value(&args, "--keypair").unwrap_or("keys/keypair");

    info!("--- INICIANDO NÓ ATLASDB ---");
    info!("Config: {}", config_path);
    info!("Endereço P2P: {}", p2p_listen_addr);
    if let Some(addr) = dial_addr { info!("Bootstrap (dial): {}", addr); }
    info!("Porta gRPC: {}", grpc_port);

    // 3. Configuração do nó
    let keypair = key_manager::load_or_generate_keypair(Path::new(keypair_path))?;
    let auth = Arc::new(RwLock::new(Ed25519Authenticator::new(keypair)));
    let p2p_config = P2pConfig {
        listen_multiaddrs: vec![p2p_listen_addr.into()],
        bootstrap: dial_addr.map(|addr| vec![addr.into()]).unwrap_or_default(),
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

/// Helper para parsear argumentos simples no formato --key value
fn get_arg_value<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|pos| args.get(pos + 1))
        .map(|s| s.as_str())
}
