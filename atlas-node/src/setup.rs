use tracing::{info, error};
use std::path::Path;
use crate::config::Config;
use atlas_common::utils::NodeId;

pub fn ensure_config(path: &str, listen_addr: &str) -> Result<(), Box<dyn std::error::Error>> {
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

pub fn setup_upnp(p2p_port: u16, grpc_port: u16) {
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

