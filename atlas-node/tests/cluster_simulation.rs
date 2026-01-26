use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use atlas_node::runtime::consensus_driver::ConsensusDriver;
use atlas_p2p::ports::P2pPublisher;
use atlas_node::config::Config;
use atlas_common::auth::ed25519::Ed25519Authenticator; // Removed unused Authenticator trait import if not needed, or keep
use atlas_common::auth::Authenticator; 
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use async_trait::async_trait;
use atlas_ledger::storage::Storage;
use atlas_common::env::vote_data::VoteData; 

#[derive(Clone)]
struct MockPublisher {
    sent: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
}

#[async_trait]
impl P2pPublisher for MockPublisher {
    async fn publish(&self, topic: &str, data: Vec<u8>) -> Result<(), String> {
        self.sent.lock().await.push((topic.to_string(), data));
        Ok(())
    }
    async fn send_response(&self, _req_id: u64, _res: atlas_p2p::protocol::TxBundle) -> Result<(), String> { Ok(()) }
    async fn request_state(&self, _peer: libp2p::PeerId, _height: u64) -> Result<(), String> { Ok(()) }
}

#[tokio::test]
async fn test_consensus_driver_basic_flow() {
    // 1. Setup Auth
    let mut csprng = OsRng;
    let keypair = SigningKey::generate(&mut csprng);
    let auth = Arc::new(RwLock::new(Ed25519Authenticator::new(keypair)));

    // 2. Setup Config
    let temp_dir = tempfile::tempdir().unwrap();
    let data_dir = temp_dir.path().to_str().unwrap().to_string();
    
    let storage = Storage::new_detached();

    let config = Config {
         node_id: atlas_common::utils::NodeId("node1".to_string()),
         address: "127.0.0.1".to_string(),
         port: 3000,
         quorum_policy: atlas_consensus::QuorumPolicy::default(),
         graph: Default::default(),
         storage,
         peer_manager: atlas_p2p::PeerManager::new(10, 10),
         data_dir,
    };

    // 3. Build Cluster
    let cluster = Arc::new(config.build_cluster_env(auth).await);

    // 4. Setup Driver
    let publisher = MockPublisher { sent: Arc::new(Mutex::new(Vec::new())) };
    let mempool = Arc::new(atlas_mempool::Mempool::default());
    let driver = ConsensusDriver::new(cluster.clone(), publisher.clone(), mempool);

    // 5. Simulate receiving a Proposal
    // Create a proposal manually
    let proposal_id = "prop-test-1".to_string();
    let proposer_id = atlas_common::utils::NodeId("node_x".to_string());
    
    // Calculate State Root
    let height = 1u64;
    let prev_hash = "0000".to_string();
    let proposer_str = proposer_id.to_string();
    
    let expected_root = {
        use sha2::{Digest, Sha256};
        let mut leaves_map = std::collections::BTreeMap::new();
        leaves_map.insert("height", height.to_be_bytes().to_vec());
        leaves_map.insert("prev_hash", prev_hash.as_bytes().to_vec());
        leaves_map.insert("proposer", proposer_str.as_bytes().to_vec());

        let leaves: Vec<Vec<u8>> = leaves_map.iter().map(|(k, v)| {
            let mut hasher = Sha256::new();
            hasher.update(k.as_bytes());
            hasher.update(v);
            hasher.finalize().to_vec()
        }).collect();

        // Use atlas_common helper if available, else standard merkle root calc
        atlas_common::crypto::merkle::calculate_merkle_root(&leaves)
    };

    let mut proposal = atlas_common::env::proposal::Proposal {
        id: proposal_id.clone(),
        proposer: proposer_id,
        content: "[]".to_string(), // Empty batch
        parent: None,
        height,
        hash: "test_hash".to_string(),
        prev_hash: prev_hash.clone(),
        round: 0,
        time: 0,
        state_root: expected_root,
        signature: [0u8; 64],
        public_key: vec![],
    };
    
    // Sign the proposal
    let mut csprng = OsRng;
    let node_x_keypair = SigningKey::generate(&mut csprng);
    let node_x_auth = Ed25519Authenticator::new(node_x_keypair);
    
    proposal.public_key = node_x_auth.public_key();
    let sign_bytes = atlas_common::env::proposal::signing_bytes(&proposal);
    let signature_vec = node_x_auth.sign(sign_bytes).unwrap();
    proposal.signature = signature_vec.try_into().unwrap();

    let bytes = bincode::serialize(&proposal).unwrap();

    // Call handle_proposal
    driver.handle_proposal(bytes).await;

    // 6. Assert Prepare Vote was Broadcasted
    let sent = publisher.sent.lock().await;
    assert!(!sent.is_empty(), "Should have sent a vote");
    let (topic, data) = &sent[0];
    assert_eq!(topic, "atlas/vote/v1");
    
    let vote: VoteData = bincode::deserialize(data).unwrap();
    assert_eq!(vote.proposal_id, proposal_id);
    assert!(matches!(vote.phase, atlas_common::env::consensus::types::ConsensusPhase::Prepare));
    
    println!("✅ Verified: Received Proposal -> Broadcasted Prepare Vote");
}
