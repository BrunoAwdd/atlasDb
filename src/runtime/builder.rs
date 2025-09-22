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
) -> Result<AtlasRuntime> {
    let config = Config::load_from_file(config_path)?;
    let cluster = Arc::new(config.build_cluster_env(auth));

    // 2) Canais P2P
    let (adapter_evt_tx, maestro_evt_rx) = mpsc::channel::<AdapterEvent>(64);
    let (maestro_cmd_tx, adapter_cmd_rx) = mpsc::channel::<AdapterCmd>(32);

    // 3) Adapter (Libp2p) + spawn
    let adapter = Libp2pAdapter::new(p2p_cfg, adapter_evt_tx, adapter_cmd_rx)
        .await
        .map_err(|e| AtlasError::Other(format!("p2p init: {e}")))?;
    tokio::spawn(async move { adapter.run().await });

    // 4) Porta (publisher) e Maestro
    let publisher = AdapterHandle { cmd_tx: maestro_cmd_tx };
    let maestro = Maestro {
        cluster: Arc::clone(&cluster),
        p2p: publisher.clone(), // AdapterHandle implementa P2pPublisher
        evt_rx: Mutex::new(maestro_evt_rx),
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

    // Exemplo p2p config (ajuste conforme sua CLI / arquivo):
    let p2p_cfg = P2pConfig {
        listen_multiaddrs: vec!["/ip4/0.0.0.0/tcp/4001".into()],
        bootstrap: vec![],
        enable_mdns: true,
        enable_kademlia: true,
    };

    let _rt = build_runtime("config.json", auth, p2p_cfg).await?;

    // Bloqueia o processo (até ter shutdown)
    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }
}


