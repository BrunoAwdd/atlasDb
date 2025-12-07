use std::sync::Arc;
use tokio::sync::mpsc;
use atlas_db::network::in_memory::InMemoryNetwork;
use atlas_db::network::message::ClusterMessage;
use atlas_db::network::traits::Network;
use atlas_sdk::utils::NodeId;
use atlas_sdk::env::proposal::Proposal;

#[tokio::test]
async fn test_in_memory_network() {
    let node_a_id = NodeId::from("node-A");
    let node_b_id = NodeId::from("node-B");
    let node_c_id = NodeId::from("node-C");

    let (net_a, tx_a, mut rx_a) = InMemoryNetwork::new(node_a_id.clone());
    let (net_b, tx_b, mut rx_b) = InMemoryNetwork::new(node_b_id.clone());
    let (net_c, tx_c, mut rx_c) = InMemoryNetwork::new(node_c_id.clone());

    // Connect peers
    net_a.add_peer(node_b_id.clone(), tx_b.clone());
    net_a.add_peer(node_c_id.clone(), tx_c.clone());

    net_b.add_peer(node_a_id.clone(), tx_a.clone());
    net_b.add_peer(node_c_id.clone(), tx_c.clone());

    net_c.add_peer(node_a_id.clone(), tx_a.clone());
    net_c.add_peer(node_b_id.clone(), tx_b.clone());

    // Start listeners
    // We need to set handlers first.
    let received_msgs_b = Arc::new(std::sync::Mutex::new(Vec::new()));
    let received_msgs_b_clone = received_msgs_b.clone();
    net_b.set_message_handler(Box::new(move |msg| {
        received_msgs_b_clone.lock().unwrap().push(msg);
    }));

    let received_msgs_c = Arc::new(std::sync::Mutex::new(Vec::new()));
    let received_msgs_c_clone = received_msgs_c.clone();
    net_c.set_message_handler(Box::new(move |msg| {
        received_msgs_c_clone.lock().unwrap().push(msg);
    }));

    // Spawn runners
    let net_b_handle = net_b.clone();
    tokio::spawn(async move {
        net_b_handle.run(rx_b).await;
    });

    let net_c_handle = net_c.clone();
    tokio::spawn(async move {
        net_c_handle.run(rx_c).await;
    });

    // Test send_to
    let msg = ClusterMessage::Proposal(Proposal {
        id: "prop-1".to_string(),
        parent: None,
        proposer: node_a_id.clone(),
        content: "content".to_string(),
        signature: [0u8; 64],
        public_key: vec![],
        height: 0,
    });

    net_a.send_to(node_b_id.clone(), msg.clone()).await.expect("Failed to send to B");

    // Give some time for processing
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    {
        let msgs = received_msgs_b.lock().unwrap();
        assert_eq!(msgs.len(), 1);
    }

    // Test broadcast
    let msg2 = ClusterMessage::Proposal(Proposal {
        id: "prop-2".to_string(),
        parent: None,
        proposer: node_a_id.clone(),
        content: "content2".to_string(),
        signature: [0u8; 64],
        public_key: vec![],
        height: 1,
    });

    net_a.broadcast(msg2).await.expect("Failed to broadcast");

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    {
        let msgs_b = received_msgs_b.lock().unwrap();
        assert_eq!(msgs_b.len(), 2); // 1 from before + 1 broadcast
    }

    {
        let msgs_c = received_msgs_c.lock().unwrap();
        assert_eq!(msgs_c.len(), 1); // 1 broadcast
    }
}

// Helper to make InMemoryNetwork cloneable (it already is if fields are Arcs, but let's verify)
// The struct definition:
// pub struct InMemoryNetwork {
//     pub id: NodeId,
//     peers: Arc<Mutex<HashMap<NodeId, Sender<ClusterMessage>>>>,
//     message_handler: Arc<Mutex<Option<Box<dyn Fn(ClusterMessage) + Send + Sync>>>>,
// }
// NodeId is String wrapper, so Clone.
// Arcs are Clone.
// So we just need to derive Clone on the struct in `in_memory.rs`.
