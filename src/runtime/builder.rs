use std::{sync::Arc, time::Duration};
use bincode::config;
use tokio::sync::{mpsc, Mutex};

use crate::error::AtlasError;
pub type Result<T> = std::result::Result<T, AtlasError>;

use crate::{
    auth::Authenticator,
    cluster::{builder::ClusterBuilder, core::Cluster},
    env::config::EnvConfig,
    network::p2p::{
        adapter::{AdapterCmd, Libp2pAdapter},
        config::P2pConfig,
        events::AdapterEvent,
        ports::{AdapterHandle, P2pPublisher}, // trait opcional p/ abstrair
    },
    runtime::maestro::Maestro,
    config::Config,
};

pub struct AtlasRuntime {
    pub cluster: Arc<Cluster>,
    pub publisher: AdapterHandle,
    // se quiser poder encerrar depois, guarde os JoinHandles:
    // pub adapter_task: tokio::task::JoinHandle<()>,
    // pub maestro_task: tokio::task::JoinHandle<()>,
}

impl AtlasRuntime {
    pub async fn send_proposals(&self) -> Result<()> {
        let proposals = self.cluster.get_proposals()
            .await.map_err(|e| AtlasError::Other(e.to_string()))?;
        for p in proposals {
            self.publisher.publish(&p.id, p.bytes())
                .await.map_err(|e| AtlasError::Other(e.to_string()))?;
        }

        Ok(())
    }

    pub async fn send_votes(&self) -> Result<()> {
        let votes = self.cluster.vote_proposals()
            .await.map_err(|e| AtlasError::Other(e.to_string()))?;
        for v in votes {
            self.publisher.publish(&v.proposal_id, v.bytes())
                .await.map_err(|e| AtlasError::Other(e.to_string()))?;
        }
        Ok(())
    }


}

pub async fn build_runtime(
    config_path: &str,
    auth: Arc<tokio::sync::RwLock<dyn Authenticator>>,
    p2p_cfg: P2pConfig,
    grpc_addr: std::net::SocketAddr,
) -> Result<AtlasRuntime> {
    let config = Config::load_from_file(config_path)?;
    let cluster = Arc::new(config.build_cluster_env(auth));

    // 2) Canais P2P
    let (adapter_evt_tx, maestro_evt_rx) = mpsc::channel::<AdapterEvent>(64);
    let (maestro_cmd_tx, adapter_cmd_rx) = mpsc::channel::<AdapterCmd>(32);

    // 3) Adapter (Libp2p) + spawn
    let peer_manager = Arc::clone(&cluster.peer_manager);
    let adapter = Libp2pAdapter::new(p2p_cfg, adapter_evt_tx, adapter_cmd_rx, peer_manager)
        .await
        .map_err(|e| AtlasError::Other(format!("p2p init: {e}")))?;

    let local_node_id = adapter.peer_id.into();
    cluster.local_node.write().await.id = local_node_id;

    tokio::spawn(async move { adapter.run().await });

    // 4) Porta (publisher) e Maestro
    let publisher = AdapterHandle { cmd_tx: maestro_cmd_tx };
    let maestro = Maestro {
        cluster: Arc::clone(&cluster),
        p2p: publisher.clone(), // AdapterHandle implementa P2pPublisher
        evt_rx: Mutex::new(maestro_evt_rx),
        grpc_addr,
        grpc_server_handle: Mutex::new(None),
    };
    let maestro = Arc::new(maestro);
    let m = Arc::clone(&maestro);
    tokio::spawn(async move { m.run().await });

    Ok(AtlasRuntime { cluster, publisher })
}

pub async fn run_cli() -> Result<()> {
    // Exemplo: configs mínimas
    let auth = Arc::new(tokio::sync::RwLock::new(
        crate::auth::authenticator::SimpleAuthenticator::new(vec![]),
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


