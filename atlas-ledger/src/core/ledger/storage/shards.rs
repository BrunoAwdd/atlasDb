use std::path::PathBuf;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use atlas_common::error::Result;
use atlas_common::entry::LedgerEntry;

#[derive(Debug)]
pub struct ShardStorage {
    base_path: PathBuf,
}

impl ShardStorage {
    pub async fn new(data_dir: &str) -> Result<Self> {
        let base_path = PathBuf::from(data_dir).join("accounts");
        fs::create_dir_all(&base_path).await?;
        Ok(Self { base_path })
    }

    pub async fn append(&self, account: &str, entry: &LedgerEntry) -> Result<()> {
        // Flat Storage: Just {account}.bin
        // Sanitize: Replace ':' with '_' to prevent IO errors on some filesystems
        let safe_filename = account.replace(":", "_");
        let file_path = self.base_path.join(format!("{}.bin", safe_filename));
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;

        // Simple length-prefixed serialization or JSON Line for now? 
        // User asked for "multiarquivos", let's do readable JSON for prototype debugging, 
        // or binary for "Staking Proof" compactness. 
        // Let's stick to JSON lines for inspectability as "files independentes".
        let mut data = serde_json::to_vec(entry)?;
        data.push(b'\n'); // Newline delimiter

        file.write_all(&data).await?;
        file.flush().await?;

        Ok(())
    }
}
