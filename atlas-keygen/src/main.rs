use clap::{Parser, Subcommand};
use libp2p::identity::Keypair;
use std::path::PathBuf;
use std::fs;

#[derive(Parser)]
#[command(name = "atlas-keygen")]
#[command(about = "AtlasDB Key Management Tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract Public Key Address from a keypair file
    Extract {
        #[arg(value_name = "FILE")]
        path: PathBuf,
    },
    /// Generate a new keypair (TODO)
    Generate {
        #[arg(short, long, value_name = "OUT")]
        out: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract { path } => {
            match fs::read(&path) {
                Ok(mut bytes) => {
                    match Keypair::from_protobuf_encoding(&bytes) {
                        Ok(keypair) => {
                             let ed_key = keypair.clone().try_into_ed25519().expect("Not an Ed25519 key");
                             let pub_key = ed_key.public();
                             let pub_bytes = pub_key.to_bytes();
                             let addr = bs58::encode(pub_bytes).into_string();
                             println!("Address: {}", addr);
                             println!("PubHex: {}", hex::encode(pub_bytes));
                             
                             let peer_id = keypair.public().to_peer_id();
                             println!("PeerId: {}", peer_id);
                             println!("PeerIdHex: {}", hex::encode(peer_id.to_bytes()));
                        },
                        Err(e) => eprintln!("Failed to decode keypair: {}", e),
                    }
                },
                Err(e) => eprintln!("Failed to read file: {}", e),
            }
        },
        Commands::Generate { out } => {
            println!("Generation not implemented yet. Use 'extract' for now.");
        }
    }
}
