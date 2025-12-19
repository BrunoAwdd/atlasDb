use atlas_common::{
    auth::{
        ed25519::Ed25519Authenticator,

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

use tracing::{info, error};

use std::path::Path;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Inicializar o logger
    let args: Vec<String> = std::env::args().collect();

    // PANIC HOOK (Windows Debugging)
    std::panic::set_hook(Box::new(|info| {
        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            },
        };
        let location = match info.location() {
            Some(l) => format!("at {}:{}:{}", l.file(), l.line(), l.column()),
            None => "unknown location".to_string(),
        };
        let err_msg = format!("CRASH: {} {}\n", msg, location);
        eprintln!("{}", err_msg);
        let _ = std::fs::write("panic.log", err_msg);
    }));
    
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

    let log_filename = format!("logs/audit-{}.log", node_name);

    let file_appender = tracing_appender::rolling::never(".", log_filename);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let consensus_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
            metadata.target() == "consensus" || 
            metadata.target().starts_with("atlas_ledger") || 
            metadata.target().starts_with("atlas_node")
        }));

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_filter(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,atlas_node=debug".into()))
        .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
            metadata.target() != "consensus"
        }));

    tracing_subscriber::registry()
        .with(consensus_layer)
        .with(stdout_layer)
        .init();

    info!("--- INICIANDO NÓ ATLASDB ---");
    info!("Config: {}", config_path);
    info!("Endereço P2P: {}", p2p_listen_addr);
    if let Some(addr) = dial_addr { info!("Bootstrap (dial): {}", addr); }
    info!("Porta gRPC: {}", grpc_port);

    // 2. Auto-Configuração
    if let Err(e) = ensure_config(config_path, p2p_listen_addr) {
        error!("Falha na auto-configuração: {}", e);
        return Err(e);
    }

    // 3. Configuração do nó
    use atlas_p2p::key_manager; 
    
    // Ensure keys directory exists
    if let Some(parent) = Path::new(keypair_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let keypair = key_manager::load_or_generate_keypair(Path::new(keypair_path))?;
    let auth = Arc::new(RwLock::new(convert_libp2p_keypair(keypair.clone())?));

    // Load config to get bootstrap peers
    let _config = Config::load_from_file(config_path)?;
    let mut bootstrap_peers = Vec::new();
    
    // Add CLI dial addr if present
    if let Some(addr) = dial_addr {
        bootstrap_peers.push(addr.to_string());
    }

    // Parse ports for UPnP
    let p2p_port_num = p2p_listen_addr.split('/').last().unwrap_or("0").parse::<u16>().unwrap_or(0);
    let grpc_port_num = grpc_port.parse::<u16>().unwrap_or(50051);

    // UPnP (Async/Blocking)
    tokio::task::spawn_blocking(move || {
        setup_upnp(p2p_port_num, grpc_port_num);
    });

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

fn ensure_config(path: &str, listen_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !Path::new(path).exists() {
        info!("⚠️ Config não encontrada. Gerando padrão em {}...", path);
        
        let node_id = format!("node-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());
        
        // Extract IP from /ip4/x.x.x.x/tcp/..
        let ip = listen_addr.split('/').nth(2).unwrap_or("127.0.0.1");

        let config = Config {
            node_id: NodeId(node_id),
            address: ip.to_string(),
            port: 50051,
            quorum_policy: atlas_consensus::QuorumPolicy { fraction: 0.67, min_voters: 1 },
            graph: atlas_common::env::node::Graph::new(),
            storage: atlas_ledger::storage::Storage::new_detached(),
            peer_manager: atlas_p2p::PeerManager::new(10, 10),
            data_dir: "data/db".to_string(),
        };
        config.save_to_file(path)?;
        info!("✅ Config gerada com sucesso! (IP: {})", ip);
    }
    Ok(())
}

fn setup_upnp(p2p_port: u16, grpc_port: u16) {
    if p2p_port == 0 { return; }
    info!("🔌 Tentando configurar UPnP...");

    // Detect local IP
    let local_ip = match std::net::UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => {
            if let Ok(_) = socket.connect("8.8.8.8:80") {
                if let Ok(addr) = socket.local_addr() {
                    if let std::net::IpAddr::V4(ip) = addr.ip() {
                        Some(ip)
                    } else { None }
                } else { None }
            } else { None }
        },
        Err(_) => None,
    }.unwrap_or_else(|| "0.0.0.0".parse().unwrap());

    match igd::search_gateway(Default::default()) {
        Ok(gateway) => {
            let external_ip = gateway.get_external_ip().unwrap_or_else(|_| "0.0.0.0".parse().unwrap());
            info!("🌍 IP Externo detectado: {}", external_ip);
            info!("🏠 IP Local detectado: {}", local_ip);

            let p2p_addr = std::net::SocketAddrV4::new(local_ip, p2p_port);
            match gateway.add_port(igd::PortMappingProtocol::TCP, p2p_port, p2p_addr, 0, "AtlasDB P2P") {
                Ok(_) => info!("✅ Porta P2P {} aberta com sucesso!", p2p_port),
                Err(e) => error!("❌ Falha ao abrir porta P2P {}: {}", p2p_port, e),
            }

            let grpc_addr = std::net::SocketAddrV4::new(local_ip, grpc_port);
            match gateway.add_port(igd::PortMappingProtocol::TCP, grpc_port, grpc_addr, 0, "AtlasDB gRPC") {
                Ok(_) => info!("✅ Porta gRPC {} aberta com sucesso!", grpc_port),
                Err(e) => error!("❌ Falha ao abrir porta gRPC {}: {}", grpc_port, e),
            }
        }
        Err(e) => error!("⚠️ Gateway UPnP não encontrado: {}", e),
    }
}
