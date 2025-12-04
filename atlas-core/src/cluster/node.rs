use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use atlas_sdk::utils::NodeId;

/// Represents an individual node in the cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub address: String,
    pub latency: Option<u64>,
    pub reliability_score: f32,
    last_seen: SystemTime,
}


// TODO need a better implementation
impl Node {
    pub fn new(id: NodeId, address: String, latency: Option<u64>, reliability_score: f32) -> Self {
        Node {
            id,
            address,
            latency,
            reliability_score,
            last_seen: SystemTime::now(),
        }
    }

    pub fn placeholder() -> Self {
        Self {
            id: NodeId::default(),          // se tiver impl Default
            address: String::new(),
            latency: None,
            reliability_score: 0.0,
            last_seen: std::time::SystemTime::now(),
        }
    }

    pub fn is_trusted(&self) -> bool {
        const MIN_RELIABILITY_SCORE: f32 = 0.8;
        const MAX_LATENCY: u64 = 500;
        self.reliability_score > MIN_RELIABILITY_SCORE && 
            self.latency.unwrap_or(999) < MAX_LATENCY
    }

    pub fn update_last_seen(&mut self) {
        self.last_seen = SystemTime::now();
    }

    pub fn update_latency(&mut self, v: Option<u64>) {
        self.latency = v;
    }

    pub fn get_last_seen(&self) -> SystemTime {
        self.last_seen
    }

    

}

impl Default for Node {
    fn default() -> Self {
        Self {
            id: NodeId::default(),          // requer impl Default pra NodeId
            address: String::new(),
            latency: None,
            reliability_score: 0.0,
            last_seen: SystemTime::now(),
        }
    }
}
