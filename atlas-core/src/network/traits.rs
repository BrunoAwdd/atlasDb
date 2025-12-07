use std::sync::Arc;
use async_trait::async_trait;
use atlas_sdk::utils::NodeId;
use crate::network::message::{ClusterMessage, NetworkError};

#[async_trait]
pub trait Network: Send + Sync {
    /// Sends a message to a specific peer.
    async fn send_to(&self, peer: NodeId, message: ClusterMessage) -> Result<(), NetworkError>;

    /// Broadcasts a message to all connected peers.
    async fn broadcast(&self, message: ClusterMessage) -> Result<(), NetworkError>;

    /// Returns a list of connected peers.
    async fn connected_peers(&self) -> Vec<NodeId>;

    /// Sets the handler for incoming messages.
    fn set_message_handler(&self, handler: Box<dyn Fn(ClusterMessage) + Send + Sync>);
}
