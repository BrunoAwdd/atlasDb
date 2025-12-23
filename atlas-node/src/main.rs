use atlas_common::auth::ed25519::Ed25519Authenticator;
use atlas_p2p::config::P2pConfig;
use atlas_node::{
    config::Config,
    runtime::builder::build_runtime,
    cli::Args,
    setup::{ensure_config, setup_upnp},
    api::rest::{start_rest_api, AppState},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error};
use std::path::Path;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Parse Arguments
    let args = Args::parse();
    let node_name = std::path::Path::new(&args.config_path)
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("unknown_node");

    // 2. Initialize Logging (Inline for simplicity of main context/guards)
    // PANIC HOOK
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
    info!("Config: {}", args.config_path);
    info!("Endereço P2P: {}", args.p2p_listen_addr);
    
    // 3. Setup Config
    if let Err(e) = ensure_config(&args.config_path, &args.p2p_listen_addr) {
         error!("Falha na auto-configuração: {}", e);
         return Err(e);
    }

    // 4. Load Keys & Config
    use atlas_p2p::key_manager;
    if let Some(parent) = Path::new(&args.keypair_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let keypair = key_manager::load_or_generate_keypair(Path::new(&args.keypair_path))?;
    let auth = Arc::new(RwLock::new(convert_libp2p_keypair(keypair.clone())?));
    
    // bootstrap peers
    let _config = Config::load_from_file(&args.config_path)?;
    let mut bootstrap_peers = Vec::new();
    if let Some(addr) = args.dial_addr {
        bootstrap_peers.push(addr);
    }

    // UPnP
    let p2p_port_num = args.p2p_listen_addr.split('/').last().unwrap_or("0").parse::<u16>().unwrap_or(0);
    let grpc_port_num = args.grpc_port.parse::<u16>().unwrap_or(50051);
    
    tokio::task::spawn_blocking(move || {
        setup_upnp(p2p_port_num, grpc_port_num);
    });

    let p2p_config = P2pConfig {
        listen_multiaddrs: vec![args.p2p_listen_addr.into()],
        bootstrap: bootstrap_peers,
        enable_mdns: true,
        enable_kademlia: true,
        keypair_path: args.keypair_path.clone(),
    };

    let grpc_addr_str = format!("0.0.0.0:{}", args.grpc_port);
    let grpc_addr = grpc_addr_str.parse()?;

    // 5. Start Runtime
    let runtime = match build_runtime(&args.config_path, auth, p2p_config, grpc_addr).await {
        Ok(rt) => {
            // Apply Genesis (Logic can move to dedicated genesis loader if needed, but simple enough here)
            let config_dir = std::path::Path::new(&args.config_path).parent().unwrap_or(std::path::Path::new("."));
            let genesis_path = config_dir.join("genesis.json");
            let dev_genesis_path = Path::new("example/genesis.json");
            
            let genesis_file = if genesis_path.exists() { Some(genesis_path) } 
            else if dev_genesis_path.exists() { Some(dev_genesis_path.to_path_buf()) } 
            else { None };

            if let Some(path) = genesis_file {
                 info!("🏛️ Loading Genesis from {:?}", path);
                 if let Ok(content) = std::fs::read_to_string(&path) {
                     if let Ok(genesis) = serde_json::from_str::<atlas_common::genesis::GenesisState>(&content) {
                          rt.ledger.apply_genesis_state(&genesis).await.ok();
                          info!("✅ Genesis applied successfully!");
                     }
                 }
            } else {
                 info!("⚠️ No genesis.json found. Starting with existing state or empty state.");
            }

            info!("Nó iniciado com sucesso. Pressione Ctrl+C para parar.");
            rt
        }
        Err(e) => {
            error!("Falha ao iniciar o nó: {}.", e);
            return Err(e.into());
        }
    };

    // 6. Start REST API
    let api_state = AppState {
        ledger: runtime.ledger.clone(),
        mempool: runtime.mempool.clone(),
    };
    let api_port = 3001; 
    tokio::spawn(async move {
        start_rest_api(api_port, api_state).await;
    });

    // 7. Keep Alive
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(300)).await;
    }
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
