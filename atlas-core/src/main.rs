use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::Path;
use atlas_sdk::auth::{ed25519::Ed25519Authenticator, Authenticator};
use atlas_db::network::key_manager;
use tracing::{info, error};

use atlas_db::network::p2p::config::P2pConfig;
use atlas_db::runtime::builder::build_runtime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Inicializar o logger
    // 2. Parsear argumentos da linha de comando
    let args: Vec<String> = std::env::args().collect();
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

    // 1. Inicializar o logger
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
    info!("Config: {}", config_path);
    info!("Endereço P2P: {}", p2p_listen_addr);
    if let Some(addr) = dial_addr { info!("Bootstrap (dial): {}", addr); }
    info!("Porta gRPC: {}", grpc_port);

    // 2.1 Teste manual de autenticação
    if args.contains(&"--test-auth".to_string()) {
        info!("--- MODO DE TESTE DE AUTENTICAÇÃO ---");
        let keypair = key_manager::load_or_generate_keypair(Path::new(keypair_path))?;
        let auth = convert_libp2p_keypair(keypair)?;
        
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
        return Ok(());
    }

    // 3. Configuração do nó
    let keypair = key_manager::load_or_generate_keypair(Path::new(keypair_path))?;
    let auth = Arc::new(RwLock::new(convert_libp2p_keypair(keypair.clone())?));
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

fn convert_libp2p_keypair(keypair: libp2p::identity::Keypair) -> Result<Ed25519Authenticator, Box<dyn std::error::Error>> {
    let ed25519_keypair = keypair.try_into_ed25519()
        .map_err(|_| "Keypair is not Ed25519")?;
    
    // Extract secret key bytes. 
    let secret = ed25519_keypair.secret();
    let secret_bytes = secret.as_ref();
    
    // ed25519-dalek SigningKey::from_bytes takes 32 bytes (seed).
    Ed25519Authenticator::from_bytes(secret_bytes).map_err(|e| e.into())
}
