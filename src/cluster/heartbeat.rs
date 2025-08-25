use std::{time::{SystemTime, UNIX_EPOCH}};

use super::core::Cluster;
use crate::{
    cluster_proto::{
        Ack, 
        HeartbeatMessage
    }, 
    NodeId
};

impl Cluster {
    async fn send_heartbeat(
        &self,
        to: &NodeId,
    ) -> Result<Ack, String> {
        let node_id = self.local_node.id.clone();
    
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "Failed to get system time")?
            .as_secs();
    
        // 1) Pega um snapshot do peer sem segurar lock durante o await
        let peer_snapshot = {
            let manager = self.peer_manager.read().await;
            manager
                .get_peer_stats(to)     
                .ok_or_else(|| format!("Peer {} not found", to.0))?
        };
    
        println!("⏱️ Heartbeat enviado para [{}] em [{}]", to, timestamp);
    
        // 2) Envia o heartbeat (sem locks)
        self.network
            .send_heartbeat(node_id, peer_snapshot.clone())
            .await
            .map_err(|e| {
                println!("❌ Erro de rede ao enviar heartbeat para [{}]: {:?}", to, e);
                format!("Network error: {:?}", e)
            })?;
    
        println!("✅ Heartbeat enviado com sucesso para [{}] em [{}]", to, timestamp);
    
        // 3) Atualiza as stats persistidas no PeerManager
        let mut updated = peer_snapshot;
        updated.update_last_seen(timestamp);
    
        {
            let mut manager = self.peer_manager.write().await;
            let evt = manager.update_stats(to, &updated);
            match evt {
                PeerEvent::Registered(id) => {
                    println!("📒 Peer [{}] registrado em [{}]", id.0, timestamp);
                }
                PeerEvent::Updated(id) => {
                    println!("📥 Stats do nó [{}] atualizadas para [{}]", id.0, timestamp);
                }
                PeerEvent::NoChange => { /* opcional: log */ }
                _ => {}
            }
        }
    
        Ok(Ack {
            received: true,
            message: format!("✅ Heartbeat enviado com sucesso para {}", to),
        })
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