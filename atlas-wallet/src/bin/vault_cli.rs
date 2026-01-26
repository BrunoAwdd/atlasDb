use clap::{Parser, Subcommand};
use serde::{Serialize, Deserialize};
use atlas_wallet::{
    errors::NimbleError, identity::identity::generate, 
    vault::vault::VaultData
};
use atlas_common::utils::security::generate_seed;

#[derive(Serialize, Deserialize, Debug)]
struct TestIdentity {
    username: String,
    secret_data: String,
}

#[derive(Parser)]
#[command(author = "Nimble", version = "1.0", about = "Vault CLI for Identity Bundle")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Save an identity bundle to an encrypted vault file
    Save {
        /// Path to save vault file
        #[arg(short = 'f', long)]
        path: String,

        /// Password to encrypt vault
        #[arg(short = 'p', long)]
        password: String,
    },
    /// Load an identity bundle from a vault file
    Load {
        /// Path to vault file
        #[arg(short = 'f', long)]
        path: String,

        /// Password to decrypt vault
        #[arg(short = 'p', long)]
        password: String,
    },
}

fn main() -> Result<(), NimbleError> {
    let cli = Cli::parse();

    match cli.command {
        
        Commands::Save { path, password} => {
            let seed = generate_seed();
            let bundle = match generate(&seed, password.clone()) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("❌ Falha ao gerar bundle: {:?}", e);
                    return Err(e.into());
                }
            };
            let vault = VaultData::new(1, vec![0u8; 12]);
            
            if let Err(e) = vault.save_identity_bundle(&bundle, password) {
                eprintln!("❌ Falha ao salvar o bundle no caminho '{}': {:?}", path, e);
                return Err(e.into());
            }
            println!("✅ Bundle salvo com sucesso em '{}'", path);
        }

        Commands::Load { path, password } => {
            let vault = VaultData::new(1, vec![0u8; 12]);
            let session = match vault.load_session(password.clone(), &path) {
                Ok(session) => session,
                Err(e) => {
                    eprintln!("❌ Failed to load session: {:?}", e);
                    return Err(e);
                }
            };
            println!("✅ Session carregado com sucesso: {}", session.profile.id());

        }
    }

    Ok(())
}
