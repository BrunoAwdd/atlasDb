use std::{
    collections::HashMap, 
    fs,
    io,
    path::Path, 
    sync::Arc
};

use tokio::sync::{Mutex, RwLock};
use tracing::info;

use serde::{Serialize, Deserialize};

use atlas_sdk::{
    env::{
        consensus::types::Vote, 
        consensus::types::ConsensusResult,
        proposal::Proposal
    },
    utils::NodeId
};

use crate::{
    env::{
        runtime::AtlasEnv,
        consensus::evaluator::QuorumPolicy,
    }, 
    peer_manager::PeerManager, 
    ConsensusEngine, 
    Graph, 
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
    pub votes: HashMap<String, HashMap<atlas_sdk::env::consensus::types::ConsensusPhase, HashMap<NodeId, Vote>>>,
    pub quorum_policy: QuorumPolicy,

    pub data_dir: String,
}

impl EnvConfig {
    pub fn new(
        graph: Graph, 
        storage: Storage, 
        peer_manager: PeerManager, 
        quorum_policy: QuorumPolicy, 
        proposals: Vec<Proposal>,
        votes: HashMap<String, HashMap<atlas_sdk::env::consensus::types::ConsensusPhase, HashMap<NodeId, Vote>>>,
        data_dir: String,
    ) -> Self {
        info!("📝 Criando nova configuração");

        EnvConfig {
            graph,
            storage,
            peer_manager,
            proposals,
            votes,
            quorum_policy,
            data_dir,
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

    pub fn build_env(mut self) -> AtlasEnv {
        let peer_manager = Arc::new(RwLock::new(self.peer_manager));
        let engine = ConsensusEngine::new(Arc::clone(&peer_manager), self.quorum_policy);

        // Initialize Ledger
        // Initialize Ledger
        use crate::ledger::Ledger;
        let data_dir = self.data_dir.clone();
        let ledger = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            rt.block_on(async {
                Ledger::new(&data_dir).await.expect("Failed to initialize Ledger from config")
            })
        }).join().expect("Failed to join thread");
        self.storage.ledger = Some(Arc::new(ledger));

        fn noop_callback(_: ConsensusResult) {}
        AtlasEnv {
            graph: self.graph,
            storage: Arc::new(RwLock::new(self.storage)),
            engine: Arc::new(Mutex::new(engine)),
            callback: Arc::new(noop_callback),
            peer_manager,
            data_dir: self.data_dir.clone(),
        }
    }
    
}