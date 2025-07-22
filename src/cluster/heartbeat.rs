use std::time::{SystemTime, UNIX_EPOCH};

use tokio::time::{timeout, Duration};

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
    pub async fn send_heartbeat(
        &self, 
        to: &NodeId
    ) -> Result<Ack, String> {
        let node = self.local_node.clone();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "Failed to get system time")?
            .as_secs();
    
        let peer = {
            let manager = self.peer_manager.read()
                .map_err(|_| "Failed to acquire read lock on peer manager")?;
            manager.get_peer_stats(&to)
                .ok_or_else(|| format!("Peer {} not found", to.0))?
        };
    
        let heartbeat_msg = format!(
            "{}: heartbeat from {} at {}", 
            peer.address, 
            node.id, 
            timestamp
        );
    
        // Tentativa com timeout (3 segundos)
        let result = {
            let network = self.network.write()
                .map_err(|_| "Failed to acquire write lock on network adapter")?;
    
            timeout(Duration::from_secs(3), network.send_heartbeat(
                node.id.clone(), 
                peer.clone(), 
                heartbeat_msg.clone()
            )).await
        };

        println!("⏱️ Heartbeat enviado para [{}] em [{}]", to, timestamp);
    
        match result {
            Ok(Ok(_)) => {
                println!("✅ Heartbeat enviado com sucesso para [{}] em [{}]", to, timestamp);
                Ok(Ack {
                    received: true,
                    message: format!("✅ Heartbeat enviado com sucesso para {}", to),
                })
            }
            Ok(Err(e)) => {
                println!("❌ Erro de rede ao enviar heartbeat para [{}]: {:?}", to, e);
                Err(format!("Network error: {:?}", e))
            }
            Err(_) => {
                println!("⏰ Timeout: heartbeat para [{}] demorou demais (>{}s)", to, 3);
                Err(format!("Timeout: heartbeat to {} took too long", to))
            }
        }
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
                if let Err(e) = self.send_heartbeat(&peer_id.clone()).await {
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

        let from = NodeId(msg.from);
        
        // Update peer's last seen timestamp
        if let Ok(manager) = self.peer_manager.write() {
            let peer = manager.get_peer_stats(&from);
            if let Some(mut node) = peer {
                node.update_last_seen(timestamp);
            }
        }
        
        Ack {
            received: true,
            message: format!("ACK recebido por {} em {}", self.local_node.id, timestamp),
        }
    }

}