use atlas_db::config::Config;
use atlas_db::env::consensus::evaluator::QuorumPolicy;
use atlas_db::env::storage::Storage;
use atlas_db::Graph;
use atlas_db::peer_manager::PeerManager;
use atlas_sdk::utils::NodeId;

fn main() {
    let node1_config = Config {
        node_id: NodeId("node1".to_string()),
        address: "127.0.0.1".to_string(),
        port: 3001,
        quorum_policy: QuorumPolicy::default(),
        graph: Graph::new(),
        storage: Storage::new(),
        peer_manager: PeerManager::new(10, 5),
    };
    node1_config.save_to_file("node1/config.json").unwrap();

    let node2_config = Config {
        node_id: NodeId("node2".to_string()),
        address: "127.0.0.1".to_string(),
        port: 3002,
        quorum_policy: QuorumPolicy::default(),
        graph: Graph::new(),
        storage: Storage::new(),
        peer_manager: PeerManager::new(10, 5),
    };
    node2_config.save_to_file("node2/config.json").unwrap();
}
