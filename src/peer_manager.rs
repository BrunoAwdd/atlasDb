use std::collections::{HashSet, HashMap};
use serde::{Deserialize, Serialize};

use crate::{utils::NodeId};
use crate::cluster::node::Node;

pub enum PeerCommand {
    Register(NodeId, Node),
    Drop(NodeId),
    Disconnected(NodeId),
    Rotate,
    UpdateStats(NodeId, Node),
}

pub enum PeerEvent {
    AlreadyRegistered(NodeId),
    Promoted(NodeId),
    Demoted(NodeId),
    Registered(NodeId),
    Dropped(NodeId),
    Updated(NodeId),
    NoChange,
}

#[derive(Clone,Debug, Serialize, Deserialize)]
pub struct PeerManager {
    pub active_peers: HashSet<NodeId>,
    pub reserve_peers: HashSet<NodeId>,
    pub known_peers: HashMap<NodeId, Node>,
    pub max_active: usize,
    pub max_reserve: usize,
}

impl PeerManager {
    pub fn new(max_active: usize, max_reserve: usize) -> Self {
        Self {
            active_peers: HashSet::new(),
            reserve_peers: HashSet::new(),
            known_peers: HashMap::new(),
            max_active,
            max_reserve,
        }
    }

    fn register_peer(&mut self, node_id: NodeId, stats: Node) {
        self.known_peers.insert(node_id.clone(), stats);
        if self.active_peers.contains(&node_id) || self.reserve_peers.contains(&node_id) {
            return;
        }
        if self.active_peers.len() < self.max_active {
            let _ = self.active_peers.insert(node_id);
            return;
        }
        if self.reserve_peers.len() < self.max_reserve {
            let _ = self.reserve_peers.insert(node_id);
            return;
        }
        // reserve cheia: troca o pior se o novo for melhor
        if let Some(worst_r) = self.reserve_peers.iter().min_by_key(|id| self.score_tuple(id)).cloned() {
            if self.better(&node_id, &worst_r) {
                self.reserve_peers.remove(&worst_r);
                let _ = self.reserve_peers.insert(node_id);
            }
        }
    }


    pub fn update_stats(&mut self, node_id: &NodeId, new_stats: &Node) -> PeerEvent
    where
        Node: Clone,
    {
        use std::collections::hash_map::Entry;
    
        match self.known_peers.entry(node_id.clone()) {
            Entry::Occupied(mut e) => {
                let current = e.get_mut();
                if new_stats.get_last_seen() > current.get_last_seen() {
                    current.latency = new_stats.latency;
                    current.reliability_score = new_stats.reliability_score;
                    current.update_last_seen();
                    PeerEvent::Updated(node_id.clone())
                } else {
                    PeerEvent::NoChange
                }
            }
            Entry::Vacant(v) => {
                v.insert(new_stats.clone()); // clona só aqui
                PeerEvent::Registered(node_id.clone())
            }
        }
    }


    fn drop_peer(&mut self, node_id: &NodeId) {
        self.active_peers.remove(node_id);
        self.reserve_peers.remove(node_id);
        self.known_peers.remove(node_id);
    }

    /// Rotação: promove o melhor da reserva se ele for melhor que o pior ativo (máx 1 troca)
    fn rotate_peers(&mut self) -> (Option<NodeId>, Option<NodeId>) {
        if self.active_peers.is_empty() || self.reserve_peers.is_empty() {
            return (None, None);
        }
        let worst_active = self.active_peers.iter().min_by_key(|id| self.score_tuple(id)).cloned();
        let best_reserve = self.reserve_peers.iter().max_by(|a, b| self.score_tuple(*b).cmp(&self.score_tuple(*a))).cloned();
        match (best_reserve, worst_active) {
            (Some(best_r), Some(worst_a)) if self.better(&best_r, &worst_a) => {
                self.reserve_peers.remove(&best_r);
                self.active_peers.insert(best_r.clone());
                self.active_peers.remove(&worst_a);
                self.reserve_peers.insert(worst_a.clone());
                (Some(best_r), Some(worst_a))
            }
            _ => (None, None),
        }
    }

    fn find_worst_active_peer(&self) -> Option<NodeId> {
        self.active_peers.iter().min_by_key(|id| {
            let stats = self.known_peers.get(*id);
            (
                stats.map(|s| (s.reliability_score * 100.0) as i64).unwrap_or(0),
                std::cmp::Reverse(stats.map(|s| s.latency).unwrap_or(Some(u64::MAX))),
            )
        }).cloned()
    }

    pub fn get_peer_stats(&self, id: &NodeId) -> Option<Node> {
        self.known_peers.get(id).cloned()
    }

    pub fn get_active_peers(&self) -> HashSet<NodeId> {
        self.active_peers.clone()
    }

    pub fn get_reserve_peers(&self) -> HashSet<NodeId> {
        self.reserve_peers.clone()
    }

    pub fn get_known_peers(&self) -> Vec<NodeId> {
        self.known_peers.keys().cloned().collect()
    }

    pub fn handle_command(&mut self, command: PeerCommand) -> PeerEvent {
        match &command {
            PeerCommand::Register(id, _) => log::debug!("Registering peer: {:?}", id),
            PeerCommand::Drop(id) => log::debug!("Dropping peer: {:?}", id),
            PeerCommand::Rotate => log::debug!("Rotating peers"),
            PeerCommand::UpdateStats(id, _) => log::debug!("Updating stats for peer: {:?}", id),
        }
    
        match command {
            PeerCommand::Register(id, stats) => {
                if self.known_peers.contains_key(&id) {
                    PeerEvent::AlreadyRegistered(id)
                } else {
                    self.register_peer(id.clone(), stats);
                    PeerEvent::Registered(id)
                }
            },
            PeerCommand::Drop(id) => {
                self.drop_peer(&id);
                PeerEvent::Dropped(id)
            },
            PeerCommand::Rotate => {
                let (promoted, demoted) = self.rotate_peers();
                if let Some(p) = promoted {
                    PeerEvent::Promoted(p)
                } else if let Some(d) = demoted {
                    PeerEvent::Demoted(d)
                } else {
                    PeerEvent::NoChange
                }
            },
            PeerCommand::UpdateStats(id, stats) => {
                self.update_stats(&id, &stats)
            },
        }
    }
}
