use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::Path;
use atlas_common::auth::Authenticator;
use atlas_db::network::key_manager;
use tracing::{info, error};

use atlas_db::network::p2p::config::P2pConfig;
use atlas_db::runtime::builder::build_runtime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Parse arguments
    let args = atlas_db::cli::Args::parse();
    
    // 2. Setup Logging
    let node_name = std::path::Path::new(&args.config_path)
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("unknown_node");

    let log_filename = format!("logs/consensus-{}.log", node_name);
    let file_appender = tracing_appender::rolling::never(".", log_filename);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    use tracing_subscriber::prelude::*;
    let consensus_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
            metadata.target() == "consensus"
        }));

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_filter(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,atlas_db=debug".into()));

    tracing_subscriber::registry()
        .with(consensus_layer)
        .with(stdout_layer)
        .init();

    info!("--- INICIANDO NÓ ATLASDB ---");
    info!("Config: {}", args.config_path);
    info!("Endereço P2P: {}", args.p2p_listen_addr);
    if let Some(addr) = &args.dial_addr { info!("Bootstrap (dial): {}", addr); }
    info!("Porta gRPC: {}", args.grpc_port);

    // 2.1 Test Auth
    if args.test_auth {
        return run_auth_test(&args.keypair_path);
    }

    // 3. Node Configuration
    let keypair = key_manager::load_or_generate_keypair(Path::new(&args.keypair_path))?;
    let auth = Arc::new(RwLock::new(atlas_db::utils::convert_libp2p_keypair(keypair.clone())?));

    // Load config to get bootstrap peers
    let _config = atlas_db::config::Config::load_from_file(&args.config_path)?;
    let mut bootstrap_peers = Vec::new();
    
    if let Some(addr) = args.dial_addr {
        bootstrap_peers.push(addr);
    }

    let p2p_config = P2pConfig {
        listen_multiaddrs: vec![args.p2p_listen_addr.into()],
        bootstrap: bootstrap_peers,
        enable_mdns: true,
        enable_kademlia: true,
        keypair_path: args.keypair_path.to_string(),
    };

    let grpc_addr_str = format!("0.0.0.0:{}", args.grpc_port);
    let grpc_addr = grpc_addr_str.parse()?;

    // 4. Build and run runtime
    match build_runtime(&args.config_path, auth, p2p_config, grpc_addr).await {
        Ok(_runtime) => {
            info!("Nó iniciado com sucesso. Pressione Ctrl+C para parar.");
        }
        Err(e) => {
            error!("Falha ao iniciar o nó: {}.", e);
            return Err(e.into());
        }
    }

    // 5. Keep alive
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
}

fn run_auth_test(keypair_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("--- MODO DE TESTE DE AUTENTICAÇÃO ---");
    let keypair = key_manager::load_or_generate_keypair(Path::new(keypair_path))?;
    let auth = atlas_db::utils::convert_libp2p_keypair(keypair)?;
    
    let msg = b"AtlasDB Auth Test";
    info!("Assinando mensagem: {:?}", String::from_utf8_lossy(msg));
    
    match auth.sign(msg.to_vec()) {
        Ok(sig) => {
            info!("Assinatura gerada com sucesso ({} bytes)", sig.len());
            let mut sig_arr = [0u8; 64];
            if sig.len() == 64 {
                sig_arr.copy_from_slice(&sig);
                match auth.verify(msg.to_vec(), &sig_arr) {
                    Ok(valid) => {
                        if valid {
                            info!("✅ Autenticação funcionando corretamente! Assinatura verificada.");
                        } else {
                            error!("❌ Falha na verificação da assinatura: Assinatura inválida.");
                        }
                    },
                    Err(e) => error!("❌ Erro na verificação: {}", e),
                }
            } else {
                error!("❌ Tamanho da assinatura incorreto: {}", sig.len());
            }
        },
        Err(e) => error!("❌ Falha ao assinar: {}", e),
    }
    Ok(())
}
