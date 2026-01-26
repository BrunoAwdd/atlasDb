use tokio::sync::RwLock;
use tracing::info;
use atlas_common::utils::NodeId;
use crate::peer_manager::PeerManager;
use crate::env::storage::Storage;

/// Strategy to elect the leader.
/// Currently implements a deterministic Round-Robin based on proposal height.
pub async fn elect_leader(
    local_node_id: NodeId,
    peer_manager: &RwLock<PeerManager>,
    storage: &RwLock<Storage>,
    current_leader: &RwLock<Option<NodeId>>,
) {
    let peer_manager = peer_manager.read().await;
    let active_peers = peer_manager.get_active_peers();

    let mut candidates: Vec<_> = active_peers.into_iter().collect();
    candidates.push(local_node_id.clone());
    candidates.sort(); // Deterministic order

    if candidates.is_empty() {
        let mut leader_lock = current_leader.write().await;
        if leader_lock.is_some() {
            info!("Perdeu todos os pares, abdicando da liderança.");
            *leader_lock = None;
        }
        return;
    }

    // Calculate current height (next proposal height)
    let storage_guard = storage.read().await;
    let next_height = storage_guard.proposals.len() as u64 + 1;
    drop(storage_guard);

    // Round-Robin: (Height - 1) % NumCandidates
    // Height starts at 1, so for Height 1 we want index 0.
    let index = ((next_height - 1) as usize) % candidates.len();
    let new_leader = candidates.get(index).cloned();

    let mut current_leader_lock = current_leader.write().await;
    
    // Log only if leader changes or for debug (optional)
    if *current_leader_lock != new_leader {
        info!("👑 Novo líder eleito para altura {}: {:?} (Candidatos: {:?})", next_height, new_leader, candidates);
        *current_leader_lock = new_leader;
    }
}
