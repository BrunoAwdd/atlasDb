use async_trait::async_trait;

#[async_trait]
pub trait P2pPublisher: Send + Sync {
    async fn publish(&self, topic: &str, data: Vec<u8>) -> Result<(), String>;
}

use tokio::sync::mpsc;
use crate::network::p2p::adapter::AdapterCmd;

#[derive(Clone)]
pub struct AdapterHandle {
    pub cmd_tx: mpsc::Sender<AdapterCmd>,
}

#[async_trait::async_trait]
impl P2pPublisher for AdapterHandle {
    async fn publish(&self, topic: &str, data: Vec<u8>) -> Result<(), String> {
        self.cmd_tx
            .send(AdapterCmd::Publish { topic: topic.into(), data })
            .await
            .map_err(|e| e.to_string())
    }
}
