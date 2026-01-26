use libp2p::identity::Keypair;
use std::path::PathBuf;
use std::fs;

pub fn extract_key_info(path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    let keypair = Keypair::from_protobuf_encoding(&bytes)
        .map_err(|e| format!("Failed to decode keypair: {}", e))?;

    let ed_key = keypair.clone().try_into_ed25519()
        .map_err(|_| "Not an Ed25519 key")?;
        
    let pub_key = ed_key.public();
    let pub_bytes = pub_key.to_bytes();
    let addr = bs58::encode(pub_bytes).into_string();

    println!("Address: {}", addr);
    println!("PubHex: {}", hex::encode(pub_bytes));
    
    let peer_id = keypair.public().to_peer_id();
    println!("PeerId: {}", peer_id);
    println!("PeerIdHex: {}", hex::encode(peer_id.to_bytes()));

    Ok(())
}

pub fn generate_keypair(_out: Option<PathBuf>) {
    println!("Generation not implemented yet. Use 'extract' for now.");
}
