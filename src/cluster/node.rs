use serde::{Deserialize, Serialize};

use crate::utils::NodeId;

/// Represents an individual node in the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub address: String,
    pub latency: Option<u64>,
    pub reliability_score: f32,
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
        const MIN_RELIABILITY_SCORE: f32 = 0.8;
        const MAX_LATENCY: u64 = 500;
        self.reliability_score > MIN_RELIABILITY_SCORE && 
            self.latency.unwrap_or(999) < MAX_LATENCY
    }

    pub fn update_last_seen(&mut self, timestamp: u64) {
        self.last_seen = timestamp;
    }

}