use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "atlas-keygen")]
#[command(about = "AtlasDB Key Management Tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
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
