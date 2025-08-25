// src/runtime/builder.rs
use std::{sync::Arc, time::Duration};
use tokio::sync::{oneshot, Mutex};
use tracing::{info, error};
use crate::error::AtlasError;

pub type Result<T> = std::result::Result<T, AtlasError>;

use crate::{
  cluster::{builder::ClusterBuilder, core::Cluster, service::ClusterService},
  cluster_proto::cluster_network_server::ClusterNetworkServer,
  jobs::{bus::CommandBus, scheduler::spawn_scheduler},
  cluster::command::ClusterCommand,
  env::config::EnvConfig,
  network::{adapter::NetworkAdapter, grcp_adapter::GRPCNetworkAdapter}, // ver nota #5
  auth::Authenticator,
};
use tokio::time::Duration as Dur;

pub struct AtlasRuntime {
  pub cluster: Arc<Cluster>,
  pub bus: CommandBus,
}

pub async fn build_runtime(
  config_path: &str,
  network: Option<Arc<dyn NetworkAdapter>>,
  auth: Arc<tokio::sync::RwLock<dyn Authenticator>>,
) -> Result<AtlasRuntime> {
  // carrega env/config
  let net = match network {
    Some(n) => n,
    None => {
      // address/port sairão do EnvConfig/ClusterBuilder
      Arc::new(GRPCNetworkAdapter::new("0.0.0.0".into(), 50052))
    }
  };
  let env = EnvConfig::load_from_file(config_path)?.build_env(Arc::clone(&net));
  let cluster = ClusterBuilder::new()
      .with_env(env)
      .with_network(net.clone())
      .with_auth(auth)
      .build()
      .map_err(|e| AtlasError::Other(e.to_string()))?;

  // inicia gRPC
  let arc = start_grpc(cluster).await?;
  let bus = CommandBus::new(&arc.clone(), 100, 5);

  // scheduler + heartbeat recorrente
  let sched = spawn_scheduler(bus.clone());
  let _hb = sched.enqueue_every(
      Dur::from_secs(5),
      Some(Dur::from_millis(500)),
      ClusterCommand::BroadcastHeartbeat,
  ).await;

  Ok(AtlasRuntime { cluster: arc, bus })
}

pub async fn run_cli() -> Result<()> {
  // exemplo: configurações mínimas
  // aqui você pode ler CLI/arquivo
  let auth = Arc::new(tokio::sync::RwLock::new(
      crate::auth::authenticator::SimpleAuthenticator::new(Vec::new()),
  ));

  let rt = build_runtime("config.json", None, auth).await?;
  // bloqueia aqui conforme seu modelo (ou só retorna Ok(()))
  loop { tokio::time::sleep(Duration::from_secs(60)).await; }
}

async fn start_grpc(mut cluster: Cluster) -> Result<Arc<Cluster>> {
  let addr = cluster.local_node.address.parse().map_err(|e:std::net::AddrParseError| AtlasError::Other(e.to_string()))?;
  let (tx, rx) = oneshot::channel();
  cluster.shutdown_sender = Mutex::new(Some(tx));
  let arc = Arc::new(cluster);
  let svc = ClusterService::new(arc.clone());

  info!("Starting gRPC server at {}", addr);
  tokio::spawn(async move {
      if let Err(e) = tonic::transport::Server::builder()
          .tcp_nodelay(true)
          .http2_keepalive_interval(Some(Duration::from_secs(30)))
          .add_service(ClusterNetworkServer::new(svc))
          .serve_with_shutdown(addr, async { let _ = rx.await; })
          .await
      { error!("gRPC error: {}", e); }
  });
  Ok(arc)
}
