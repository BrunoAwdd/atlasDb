use std::{
    collections::HashMap, 
    fs,
    io,
    path::Path, 
    sync::Arc
};

use tokio::sync::{Mutex, RwLock};

use serde::{Serialize, Deserialize};

use crate::{
    env::{consensus::Vote, AtlasEnv, Proposal, ConsensusResult}, 
    peer_manager::PeerManager, 
    ConsensusEngine, 
    Graph, 
    NetworkAdapter, 
    NodeId, 
    Storage 
};

#[derive(Serialize, Deserialize)]
pub struct EnvConfig {
    pub graph: Graph,
    pub storage: Storage,

    // peer manager for tracking cluster nodes
    pub peer_manager: PeerManager,
    
    // cluster engine
    pub proposals: Vec<Proposal>,
    pub votes: HashMap<String, HashMap<NodeId, Vote>>,
    pub quorum_ratio: f64,

}

impl EnvConfig {
    pub fn new(
        graph: Graph, 
        storage: Storage, 
        peer_manager: PeerManager, 
        quorum_ratio: f64, 
        proposals: Vec<Proposal>,
        votes: HashMap<String, HashMap<NodeId, Vote>>
    ) -> Self {
        println!("📝 Criando nova configuração");

        EnvConfig {
            graph,
            storage,
            peer_manager,
            proposals,
            votes,
            quorum_ratio,
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(path, json)
    }

    pub fn load_from_file<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let json = fs::read_to_string(path)?;
        let config = serde_json::from_str(&json)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(config)
    }

    pub fn build_env(self, network: Arc<RwLock<dyn NetworkAdapter>>) -> AtlasEnv {
        let peer_manager = Arc::new(RwLock::new(self.peer_manager));
        fn noop_callback(_: ConsensusResult) {}
        AtlasEnv {
            graph: self.graph,
            storage: self.storage,
            engine: ConsensusEngine::new(Arc::clone(&peer_manager), self.quorum_ratio),
            network,
            callback: Arc::new(noop_callback),
            peer_manager,
        }
    }
    
}