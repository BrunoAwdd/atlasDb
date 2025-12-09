use atlas_p2p::key_manager;
use libp2p::identity;
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <keypair_path>", args[0]);
        return;
    }
    let path = Path::new(&args[1]);
    let keypair = key_manager::load_or_generate_keypair(path).unwrap();
    let peer_id = identity::PeerId::from_public_key(&keypair.public());
    println!("{}", peer_id);
}
