use std::{fs, io, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, RwLock};
use atlas_sdk::{
    env::{
        consensus::types::ConsensusResult,
        
        node::Graph,
    },
    auth::Authenticator,
    utils::NodeId
};

use crate::{
    cluster::core::Cluster,
    env::runtime::AtlasEnv, 
    peer_manager::PeerManager,
    env::storage::Storage,
    env::consensus::evaluator::QuorumPolicy,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
// Touched for rebuild
pub struct Config {
    pub node_id: NodeId,
    pub address: String,
    pub port: u16,
    pub quorum_policy: QuorumPolicy,
    pub graph: Graph,
    pub storage: Storage,
    pub peer_manager: PeerManager,
    pub data_dir: String,
}

impl Config {
    pub fn build_cluster_env(
        self,
        auth: Arc<RwLock<dyn Authenticator>>,
    ) -> Cluster {
        let peer_manager = Arc::new(RwLock::new(self.peer_manager));
        fn noop_callback(_: ConsensusResult) {}

        let mut engine = crate::ConsensusEngine::new(
            Arc::clone(&peer_manager),
            self.quorum_policy,
        );

        for proposal in &self.storage.proposals {
            engine.pool.add(proposal.clone());
            engine.registry.register_proposal(&proposal.id);
        }

        engine.registry.replace(self.storage.votes.clone());

        // Initialize Ledger
        use crate::ledger::Ledger;
        let ledger = Ledger::new(&self.data_dir).expect("Failed to initialize Ledger from config");
        let mut storage = self.storage;
        storage.ledger = Some(Arc::new(ledger));

        let env = AtlasEnv {
            graph: self.graph,
            storage: Arc::new(RwLock::new(storage)),
            engine: Arc::new(Mutex::new(engine)),
            callback: Arc::new(noop_callback),
            peer_manager: Arc::clone(&peer_manager),
            data_dir: self.data_dir.clone(),
        };

        Cluster::new(env, self.node_id, auth)
    }

    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(path, json)
    }

    pub fn load_from_file(path: &str) -> io::Result<Self> {
        let data = std::fs::read_to_string(path)?;
        let parsed = serde_json::from_str::<Config>(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(parsed)
    }
}