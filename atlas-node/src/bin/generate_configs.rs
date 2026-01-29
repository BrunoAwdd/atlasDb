use atlas_node::config::Config;
use atlas_consensus::QuorumPolicy;
use atlas_ledger::storage::Storage;
use atlas_common::env::node::Graph;
use atlas_p2p::PeerManager;
use atlas_common::utils::NodeId;

fn main() {
    let node1_config = Config {
        node_id: NodeId("node1".to_string()),
        address: "127.0.0.1".to_string(),
        port: 3001,
        quorum_policy: QuorumPolicy::default(),
        graph: Graph::new(),
        storage: Storage::new("node1/data"),
        peer_manager: PeerManager::new(10, 5),
        data_dir: "node1/data".to_string(),
        redis_url: None,
    };
    node1_config.save_to_file("node1/config.json").unwrap();

    let node2_config = Config {
        node_id: NodeId("node2".to_string()),
        address: "127.0.0.1".to_string(),
        port: 3002,
        quorum_policy: QuorumPolicy::default(),
        graph: Graph::new(),
        storage: Storage::new("node2/data"),
        peer_manager: PeerManager::new(10, 5),
        data_dir: "node2/data".to_string(),
        redis_url: None,
    };
    node2_config.save_to_file("node2/config.json").unwrap();
}
