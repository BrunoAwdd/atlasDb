use std::env;
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use atlas_p2p::key_manager;
use atlas_p2p::config::P2pConfig;
use atlas_node::config::Config;
use atlas_ledger::storage::Storage;
use atlas_p2p::PeerManager;
use atlas_consensus::QuorumPolicy;
use atlas_common::env::node::Graph;
use atlas_common::env::node::Node;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let node_count = args.get(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(4);
    let base_port = 4000;
    let base_grpc_port = 50080;

    println!("🚀 Generating configuration for {} nodes...", node_count);

    let mut peers = Vec::new();

    let base_dir = "atlas-core/example";
    fs::create_dir_all(base_dir)?;

    // 1. Generate Keys and IDs first
    for i in 1..=node_count {
        let node_dir = format!("{}/node{}", base_dir, i);
        fs::create_dir_all(&node_dir)?;
        
        let keypair_path = Path::new(&node_dir).join("keypair");
        let keypair = key_manager::load_or_generate_keypair(&keypair_path)?;
        let peer_id = keypair.public().to_peer_id();
        
        peers.push((i, peer_id, keypair_path));
    }

    // 2. Generate Configs
    for (i, peer_id, _) in &peers {
        let node_dir = format!("{}/node{}", base_dir, i);
        let config_path = Path::new(&node_dir).join("config.json");
        
        let p2p_port = base_port + i;
        let grpc_port = base_grpc_port + i;

        // Create initial peer list (bootstrap from node1 if not node1)
        let mut known_peers = HashMap::new();
        // Everyone knows everyone for simplicity in this local setup
        for (j, pid, _) in &peers {
            if i != j {
                let addr = format!("/ip4/127.0.0.1/tcp/{}/p2p/{}", base_port + j, pid);
                let node_id = atlas_common::utils::NodeId(pid.to_string());
                let node = Node::new(
                    node_id.clone(),
                    addr,
                    None,
                    1.0, // High reliability for local test
                );
                known_peers.insert(node_id, node);
            }
        }

        let mut peer_manager = PeerManager::new(10, 5); // max_active=10, max_reserve=5
        peer_manager.known_peers = known_peers.clone();
        for (id, _) in &known_peers {
            peer_manager.reserve_peers.insert(id.clone());
        }

        // BFT Quorum Policy: f = (n-1)/3
        // For n=4, f=1. Quorum = 2f+1 = 3.
        // Fraction: 0.67 (approx 2/3)
        let f = (node_count - 1) / 3;
        let min_voters = 2 * f + 1;
        let quorum_policy = QuorumPolicy {
            fraction: 0.66, // slightly less than 2/3 to be safe with floats, or use min_voters
            min_voters,
        };

        let config = Config {
            node_id: atlas_common::utils::NodeId(peer_id.to_string()),
            address: "127.0.0.1".to_string(),
            port: p2p_port as u16,
            quorum_policy,
            graph: Graph::new(),
            storage: Storage::new(&format!("{}/data", node_dir)),
            peer_manager,
            data_dir: format!("{}/data", node_dir),
        };

        config.save_to_file(&config_path)?;
        
        println!("✅ Node {}: ID={}, P2P={}, gRPC={}", i, peer_id, p2p_port, grpc_port);
    }

    // 3. Generate Start Script
    let mut script = String::from("#!/bin/bash\n\n");
    script.push_str("trap 'kill $(jobs -p)' EXIT\n\n");
    // Get the directory where the script is located
    script.push_str("DIR=\"$( cd \"$( dirname \"${BASH_SOURCE[0]}\" )\" && pwd )\"\n");
    script.push_str("echo \"🚀 Starting AtlasDB Cluster with 4 nodes...\"\n\n");

    for (i, _peer_id, _) in &peers {
        let p2p_port = base_port + i;
        let grpc_port = base_grpc_port + i;
        let node_subdir = format!("node{}", i);
        
        // Use $DIR to make paths absolute based on script location
        let mut cmd = format!(
            "cargo run --manifest-path \"$DIR/../../Cargo.toml\" --bin atlas-node -- --listen /ip4/127.0.0.1/tcp/{} --grpc-port {} --config \"$DIR/{}/config.json\" --keypair \"$DIR/{}/keypair\"",
            p2p_port, grpc_port, node_subdir, node_subdir
        );

        // Bootstrap: mDNS handles discovery now, no manual dial needed
        // if *i > 1 { ... }

        script.push_str(&format!("{} > \"$DIR/{}/node.log\" 2>&1 &\n", cmd, node_subdir));
        script.push_str(&format!("PID_{}=$!\n", i));
        script.push_str(&format!("echo \"Started Node {} (PID $PID_{})\"\n", i, i));

        if *i == 1 {
            script.push_str("sleep 5\n");
        }
    }

    script.push_str("\nwait\n");
    
    let script_path = format!("{}/start_cluster.sh", base_dir);
    fs::write(&script_path, script)?;
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755))?;

    println!("\n✨ Cluster setup complete! Run ./{}/start_cluster.sh to start.", base_dir);

    Ok(())
}
