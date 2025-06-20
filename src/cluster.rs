//! cluster.rs
//!
//! Simulated cluster environment for peer-to-peer communication between distributed nodes.
//!
//! This module provides logical constructs to mimic inter-node messaging, heartbeat cycles,
//! and rudimentary graph updates — all without actual networking.
//!
//! ⚠️ Note: This is a simulation layer, not a real distributed system.

use std::collections::HashMap;
use std::thread;
use std::time::Duration;

use crate::utils::NodeId;

/// Messages exchanged between cluster nodes.
#[derive(Debug, Clone)]
pub enum ClusterMessage {
    /// Periodic heartbeat signal indicating liveness.
    Heartbeat(NodeId),

    /// A graph mutation proposal or update.
    GraphUpdate(NodeId, String),

    /// Acknowledgment of receipt or approval.
    Acknowledge(NodeId),
}

/// Represents an individual node in the cluster.
pub struct ClusterNode {
    /// Unique identifier for the node.
    pub id: NodeId,

    /// Queue of incoming messages to be processed.
    pub inbox: Vec<ClusterMessage>,
}

impl ClusterNode {
    /// Constructs a new node with the given identifier.
    pub fn new(id: NodeId) -> Self {
        ClusterNode {
            id,
            inbox: Vec::new(),
        }
    }

    /// Produces a heartbeat message to signal activity to peers.
    pub fn send_heartbeat(&self) -> ClusterMessage {
        ClusterMessage::Heartbeat(self.id.clone())
    }

    /// Processes all pending messages in the inbox.
    /// This is where a node would apply logic to react to received messages.
    pub fn process_inbox(&mut self) {
        for msg in self.inbox.drain(..) {
            match msg {
                ClusterMessage::Heartbeat(sender) => {
                    println!("🧭 [{}] received heartbeat from [{}]", self.id, sender);
                }
                ClusterMessage::GraphUpdate(sender, payload) => {
                    println!(
                        "🔗 [{}] received graph update from [{}]: {}",
                        self.id, sender, payload
                    );
                }
                ClusterMessage::Acknowledge(sender) => {
                    println!("📨 [{}] received ACK from [{}]", self.id, sender);
                }
            }
        }
    }
}

/// Simulates a distributed cluster composed of multiple nodes.
///
/// This structure provides mechanisms for broadcasting messages,
/// simulating inter-node communication, and running cyclical simulations.
pub struct Cluster {
    /// The full set of nodes currently part of the cluster.
    pub nodes: HashMap<NodeId, ClusterNode>,
}

impl Cluster {
    /// Initializes a new, empty cluster.
    pub fn new() -> Self {
        Cluster {
            nodes: HashMap::new(),
        }
    }

    /// Adds a new node to the cluster by its unique identifier.
    pub fn add_node(&mut self, id: NodeId) {
        let node = ClusterNode::new(id.clone());
        self.nodes.insert(id, node);
    }

    /// Broadcasts heartbeat messages from all nodes to all other peers.
    ///
    /// This simulates a simple full-mesh heartbeat exchange.
    pub fn broadcast_heartbeats(&mut self) {
        let node_ids: Vec<NodeId> = self.nodes.keys().cloned().collect();

        for id in &node_ids {
            if let Some(sender_node) = self.nodes.get(id) {
                let hb = sender_node.send_heartbeat();

                for target_id in &node_ids {
                    if target_id != id {
                        if let Some(receiver) = self.nodes.get_mut(target_id) {
                            receiver.inbox.push(hb.clone());
                        }
                    }
                }
            }
        }

        for node in self.nodes.values_mut() {
            node.process_inbox();
        }
    }

    /// Runs a full simulation loop, broadcasting heartbeats at fixed intervals.
    ///
    /// # Parameters
    /// - `cycles`: Number of iterations to simulate (1 per second).
    pub fn run_simulation(&mut self, cycles: u32) {
        println!("⏳ Simulating {} heartbeat cycles...\n", cycles);
        for cycle in 1..=cycles {
            println!("--- Cycle {} ---", cycle);
            self.broadcast_heartbeats();
            thread::sleep(Duration::from_millis(1000));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::NodeId;

    fn create_cluster_with_nodes(n: usize) -> Cluster {
        let mut cluster = Cluster::new();
        for i in 0..n {
            let node_id = NodeId(format!("node-{}", i));
            cluster.add_node(node_id);
        }
        cluster
    }

    #[test]
    fn test_node_initialization() {
        let node_id = NodeId("node-A".to_string());
        let node = ClusterNode::new(node_id.clone());

        assert_eq!(node.id, node_id);
        assert!(node.inbox.is_empty());
    }

    #[test]
    fn test_add_nodes_to_cluster() {
        let cluster = create_cluster_with_nodes(3);
        assert_eq!(cluster.nodes.len(), 3);
    }

    #[test]
    fn test_heartbeat_broadcast() {
        let mut cluster = create_cluster_with_nodes(3);

        cluster.broadcast_heartbeats();

        // Every node should receive heartbeats from the other two nodes
        for (id, node) in cluster.nodes.iter() {
            assert_eq!(
                node.inbox.len(),
                0,
                "Inbox for node {} should have been processed",
                id
            );
        }
    }

    #[test]
    fn test_heartbeat_message_format() {
        let node_id = NodeId("node-X".to_string());
        let node = ClusterNode::new(node_id.clone());

        let msg = node.send_heartbeat();
        match msg {
            ClusterMessage::Heartbeat(sender) => assert_eq!(sender, node_id),
            _ => panic!("Expected Heartbeat message"),
        }
    }

    #[test]
    fn test_simulation_cycles() {
        let mut cluster = create_cluster_with_nodes(2);

        // Override sleep to avoid slowing down test
        for _ in 0..2 {
            cluster.broadcast_heartbeats();
        }

        // Inbox should be empty after processing in each cycle
        for node in cluster.nodes.values() {
            assert!(node.inbox.is_empty());
        }
    }
}
