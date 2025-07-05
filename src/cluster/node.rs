use serde::{Deserialize, Serialize};

use crate::utils::NodeId;

/// Represents an individual node in the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub address: String, // pode ser um endpoint, ou ID de rede
    pub latency: Option<u64>, // em ms, para priorização
    pub reliability_score: f32, // para o PeerManager
    pub last_seen: u64,
}

impl Node {
    pub fn new(id: NodeId, address: String, latency: Option<u64>, reliability_score: f32) -> Self {
        Node {
            id,
            address,
            latency,
            reliability_score,
            last_seen: 0,
        }
    }

    pub fn is_trusted(&self) -> bool {
        self.reliability_score > 0.8 && self.latency.unwrap_or(999) < 500
    }

}