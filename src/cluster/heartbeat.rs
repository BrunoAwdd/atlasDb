use std::time::{SystemTime, UNIX_EPOCH};

use super::core::Cluster;
use crate::{
    cluster_proto::{
        Ack, 
        HeartbeatMessage
    }, 
    NodeId
};

// TODO: Implement timeouts for heartbeats
impl Cluster {
    pub async fn send_heartbeat(&self, to: NodeId, msg: String) -> Result<Ack, String> {
        let ack = self
            .local_env
            .write()
            .map_err(|_| "Failed to acquire write lock on local env")?
            .send_heartbeat(self.local_node.clone(), &to)
            .await
            .map_err(|e| format!("Failed to send heartbeat: {}", e))?;

        Ok(ack)
    }

     /// Broadcasts heartbeat messages from all nodes to all other peers.
    pub async fn broadcast_heartbeats(&self) -> Result<(), String> {
        let peers = {
            let manager = self.peer_manager.read()
                .map_err(|_| "Failed to acquire read lock on peer manager")?;
            manager.get_active_peers().iter().cloned().collect::<Vec<NodeId>>()
        };
        
        let sender_id = self.local_node.id.clone();
        let mut errors = Vec::new();

        for peer_id in peers {
            if peer_id != sender_id {
                if let Err(e) = self.send_heartbeat(peer_id.clone(), "broadcast".to_string()).await {
                    errors.push(format!("Failed to send heartbeat to {}: {}", peer_id, e));
                }
            }
        }

        if !errors.is_empty() {
            return Err(format!("Some heartbeats failed: {}", errors.join(", ")));
        }
        
        Ok(())
    }

       /// Handles incoming heartbeat messages
    pub fn handle_heartbeat(&self, msg: HeartbeatMessage) -> Ack {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        println!("⏱️ Heartbeat recebido de [{}] em [{}]", msg.from, msg.timestamp);
        
        // Update peer's last seen timestamp
        if let Ok(mut manager) = self.peer_manager.write() {
            // Note: This would need a method to update last_seen in PeerManager
            // manager.update_peer_last_seen(&msg.from, timestamp);
        }
        
        Ack {
            received: true,
            message: format!("ACK recebido por {} em {}", self.local_node.id, timestamp),
        }
    }

}