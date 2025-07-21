use crate::{peer_manager::PeerCommand, Cluster, Node, NodeId};

impl Cluster {
    /// Adds a new node to the cluster by its unique identifier.
    pub fn add_node(&mut self, id: NodeId, stats: Node) -> Result<(), String> {
        let cmd = PeerCommand::Register(id, stats);
        let mut manager = self.peer_manager.write()
            .map_err(|_| "Failed to acquire write lock on peer manager")?;
        manager.handle_command(cmd);
        Ok(())
    }

    /// Gets the number of active peers in the cluster
    pub fn get_peer_count(&self) -> Result<usize, String> {
        let manager = self.peer_manager.read()
            .map_err(|_| "Failed to acquire read lock on peer manager")?;
        Ok(manager.get_active_peers().len())
    }

    /// Checks if a specific peer is active
    pub fn is_peer_active(&self, peer_id: &NodeId) -> Result<bool, String> {
        let manager = self.peer_manager.read()
            .map_err(|_| "Failed to acquire read lock on peer manager")?;
        Ok(manager.get_peer_stats(peer_id).is_some())
    }

}