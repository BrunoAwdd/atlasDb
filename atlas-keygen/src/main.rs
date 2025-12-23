mod cli;
mod operations;

use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Extract { path } => {
            if let Err(e) = operations::extract_key_info(&path) {
                eprintln!("Error extracting key info: {}", e);
                std::process::exit(1);
            }
        },
        Commands::Generate { out } => {
            operations::generate_keypair(out);
        }
    }
}
