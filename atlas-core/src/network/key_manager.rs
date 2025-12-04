use libp2p::identity;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

/// Loads a keypair from a file or generates a new one if the file does not exist.
///
/// # Arguments
///
/// * `path` - The path to the keypair file.
///
/// # Returns
///
/// A `Result` containing the `identity::Keypair` or an `io::Error`.
pub fn load_or_generate_keypair(path: &Path) -> io::Result<identity::Keypair> {
    if path.exists() {
        let mut file = fs::File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        identity::Keypair::from_protobuf_encoding(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    } else {
        let keypair = identity::Keypair::generate_ed25519();
        let bytes = keypair
            .to_protobuf_encoding()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::File::create(path)?;
        file.write_all(&bytes)?;
        Ok(keypair)
    }
}