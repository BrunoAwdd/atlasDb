use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, Mutex};

use atlas_common::error::AtlasError;
pub type Result<T> = std::result::Result<T, AtlasError>;

use atlas_common::auth::Authenticator;

use atlas_consensus::Cluster;

use atlas_p2p::{
    adapter::{AdapterCmd, Libp2pAdapter},
    config::P2pConfig,
    events::AdapterEvent,
    ports::AdapterHandle,
};


use crate::runtime::maestro::Maestro;
use crate::config::Config;

pub struct AtlasRuntime {
    pub cluster: Arc<Cluster>,
    pub publisher: AdapterHandle,
    pub ledger: Arc<atlas_ledger::Ledger>,
    pub mempool: Arc<atlas_mempool::Mempool>,
}

impl AtlasRuntime {
// ...
}

pub async fn build_runtime(
    config_path: &str,
    auth: Arc<tokio::sync::RwLock<dyn Authenticator>>,
    p2p_cfg: P2pConfig,
    grpc_addr: std::net::SocketAddr,
) -> Result<AtlasRuntime> {
    let config = Config::load_from_file(config_path)?;
    tracing::info!("🔄 [DEBUG] Config loaded. Building cluster env...");
    
    let cluster = Arc::new(config.build_cluster_env(auth).await);
    tracing::info!("✅ [DEBUG] Cluster env built (Ledger init success).");

    // Extract Ledger reference here
    let ledger = cluster.local_env.storage.read().await.ledger.clone()
        .ok_or_else(|| AtlasError::Other("Ledger not initialized in Cluster".to_string()))?;

    // 2) Canais P2P
    let (adapter_evt_tx, maestro_evt_rx) = mpsc::channel::<AdapterEvent>(64);
    let (maestro_cmd_tx, adapter_cmd_rx) = mpsc::channel::<AdapterCmd>(32);

    // 3) Adapter (Libp2p) + spawn
    tracing::info!("🔄 [DEBUG] Initializing P2P Adapter...");
    let peer_manager = Arc::clone(&cluster.peer_manager);
    let adapter = Libp2pAdapter::new(p2p_cfg, adapter_evt_tx, adapter_cmd_rx, peer_manager)
        .await
        .map_err(|e| AtlasError::Other(format!("p2p init: {e}")))?;
    tracing::info!("✅ [DEBUG] P2P Adapter initialized.");

    let local_node_id = adapter.peer_id.to_string().into();
    cluster.local_node.write().await.id = local_node_id;

    tokio::spawn(async move { adapter.run().await });

    // 4) Porta (publisher) e Maestro
    let publisher = AdapterHandle { cmd_tx: maestro_cmd_tx };
    // Initialize Mempool
    let mempool = Arc::new(atlas_mempool::Mempool::default());

    let maestro = Maestro {
        cluster: Arc::clone(&cluster),
        p2p: publisher.clone(), 
        mempool: Arc::clone(&mempool),
        evt_rx: Mutex::new(maestro_evt_rx),
        grpc_addr,
        grpc_server_handle: Mutex::new(None),
    };
    let maestro = Arc::new(maestro);
    let m = Arc::clone(&maestro);
    tokio::spawn(async move { m.run().await });

    Ok(AtlasRuntime { cluster, publisher, ledger, mempool })
}

pub async fn run_cli() -> Result<()> {
    // Exemplo: configs mínimas
    // Use a random key for CLI/testing if needed, or load one.
    // Since SimpleAuthenticator is gone, we use Ed25519Authenticator with a generated key.
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    
    let mut csprng = OsRng;
    let keypair = SigningKey::generate(&mut csprng);
    let auth = Arc::new(tokio::sync::RwLock::new(
        atlas_common::auth::ed25519::Ed25519Authenticator::new(keypair)
    ));

    let keypair_path = std::env::var("KEYPAIR_PATH").unwrap_or_else(|_| "keys/keypair.bin".to_string());

    // Exemplo p2p config (ajuste conforme sua CLI / arquivo):
    let p2p_cfg = P2pConfig {
        listen_multiaddrs: vec!["/ip4/0.0.0.0/tcp/4001".into()],
        bootstrap: vec![],
        enable_mdns: true,
        enable_kademlia: true,
        keypair_path,
    };

    let grpc_addr = "0.0.0.0:50051".parse().unwrap();

    let _rt = build_runtime("config.json", auth, p2p_cfg, grpc_addr).await?;

    // Bloqueia o processo (até ter shutdown)
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}


