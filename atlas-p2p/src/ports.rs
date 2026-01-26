use async_trait::async_trait;

#[async_trait]
#[async_trait]
pub trait P2pPublisher: Send + Sync {
    async fn publish(&self, topic: &str, data: Vec<u8>) -> Result<(), String>;
    async fn send_response(&self, req_id: u64, res: crate::protocol::TxBundle) -> Result<(), String>;
    async fn request_state(&self, peer: libp2p::PeerId, height: u64) -> Result<(), String>;
}

use tokio::sync::mpsc;
use crate::adapter::AdapterCmd;

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

    async fn send_response(&self, req_id: u64, res: crate::protocol::TxBundle) -> Result<(), String> {
        self.cmd_tx
            .send(AdapterCmd::SendResponse { req_id, res })
            .await
            .map_err(|e| e.to_string())
    }

    async fn request_state(&self, peer: libp2p::PeerId, height: u64) -> Result<(), String> {
        let req = crate::protocol::TxRequest::GetState { height };
        self.cmd_tx
            .send(AdapterCmd::RequestTxs { peer, req })
            .await
            .map_err(|e| e.to_string())
    }
}
