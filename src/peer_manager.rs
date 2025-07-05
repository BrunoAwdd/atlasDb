use std::collections::{HashSet, HashMap};
use serde::{Deserialize, Serialize};

use crate::{utils::NodeId};
use crate::cluster::node::Node;

pub enum PeerCommand {
    Register(NodeId, Node),
    Drop(NodeId),
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

        // Se já é ativo ou reserva, não faz nada
        if self.active_peers.contains(&node_id) || self.reserve_peers.contains(&node_id) {
            return;
        }

        // Tenta adicionar como ativo se houver espaço
        if self.active_peers.len() < self.max_active {
            self.active_peers.insert(node_id);
        } else if self.reserve_peers.len() < self.max_reserve {
            self.reserve_peers.insert(node_id);
        }
    }

    fn update_stats(&mut self, node_id: &NodeId, new_stats: Node) -> PeerEvent {
        if let Some(current) = self.known_peers.get_mut(node_id) {
            if new_stats.last_seen > current.last_seen {
                current.latency = new_stats.latency;
                current.reliability_score = new_stats.reliability_score;
                current.last_seen = new_stats.last_seen;
                PeerEvent::Updated(node_id.clone())
            } else {
                PeerEvent::NoChange
            }
        } else {
            self.known_peers.insert(node_id.clone(), new_stats);
            PeerEvent::Registered(node_id.clone())
        }
    }


    fn drop_peer(&mut self, node_id: &NodeId) {
        self.active_peers.remove(node_id);
        self.reserve_peers.remove(node_id);
        self.known_peers.remove(node_id);
    }

    fn rotate_peers(&mut self) -> (Option<NodeId>, Option<NodeId>) {
        let mut candidates: Vec<_> = self.reserve_peers.iter().cloned().collect();

        // Ordena por confiabilidade e latência
        candidates.sort_by_key(|id| {
            let stats = self.known_peers.get(id);
            (
                stats.map(|s| std::cmp::Reverse((s.reliability_score * 100.0) as i64)).unwrap_or(std::cmp::Reverse(0)),
                stats.map(|s| s.latency).unwrap_or(Some(u64::MAX)),
            )
        });

        let mut promoted = None;
        let mut demoted = None;

        // Troca o menos eficiente dos ativos se reserva for melhor
        for candidate in candidates {
            if self.active_peers.len() >= self.max_active {
                // Remove o pior dos ativos (com pior score)
                if let Some(worst) = self.find_worst_active_peer() {
                    promoted = Some(candidate.clone());
                    demoted = Some(worst.clone());
                    self.active_peers.remove(&worst);
                    self.active_peers.insert(candidate.clone());
                    self.reserve_peers.remove(&candidate);
                    self.reserve_peers.insert(worst);
                }
            }
        }

        (promoted, demoted)
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
        println!("Peer: {:?}", id);
        println!("Known: {:?}", self.known_peers);

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
                self.update_stats(&id, stats)
            },
        }
    }
}
