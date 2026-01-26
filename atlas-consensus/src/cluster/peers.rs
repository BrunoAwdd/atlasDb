use atlas_common::utils::NodeId;
use atlas_p2p::peer_manager::PeerCommand;
use atlas_common::env::node::Node;
use crate::Cluster;

impl Cluster {
    /// Adds a new node to the cluster by its unique identifier.
    pub async fn add_node(&self, id: NodeId, stats: Node) -> Result<(), String> {
        if id == self.local_node.read().await.id {
            return Ok(());
        }
        let cmd = PeerCommand::Register(id, stats);
        let mut manager = self.peer_manager.write().await;
        manager.handle_command(cmd);
        Ok(())
    }

    /// Gets the number of active peers in the cluster
    pub async fn get_peer_count(&self) -> Result<usize, String> {
        let manager = self.peer_manager.read().await;
        Ok(manager.get_active_peers().len())
    }

    /// Checks if a specific peer is active
    pub async fn is_peer_active(&self, peer_id: &NodeId) -> Result<bool, String> {
        let manager = self.peer_manager.read().await;
        Ok(manager.get_peer_stats(peer_id).is_some())
    }

}