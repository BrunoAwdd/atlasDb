use std::path::{Path, PathBuf};
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use atlas_common::env::proposal::Proposal;
use atlas_common::error::Result;

#[derive(Debug)]
pub struct Binlog {
    // current_file: File, // Removed to avoid locking issues on Windows
    current_file_id: u64,
    current_offset: u64,
    data_dir: PathBuf,
}

impl Binlog {
    pub async fn new(data_dir: &str) -> Result<Self> {
        let path = Path::new(data_dir).join("binlog");
        fs::create_dir_all(&path).await?;
        
        let file_path = path.join("00000.log");
        
        // Initialize offset checking file size
        let len = if file_path.exists() {
             let metadata = fs::metadata(&file_path).await?;
             metadata.len()
        } else {
             // Create the file explicitly to avoid "Not Found" warnings on read_all
             File::create(&file_path).await?;
             0
        };

        Ok(Self {
            current_file_id: 0,
            current_offset: len,
            data_dir: path,
        })
    }

    pub async fn append(&mut self, proposal: &Proposal) -> Result<(u64, u64, u64)> {
        let data = serde_json::to_vec(proposal)?;
        let len = data.len() as u64;
        let offset = self.current_offset;
        
        // Open file on demand to avoid holding a lock that prevents reading
        let file_path = self.data_dir.join(format!("{:05}.log", self.current_file_id));
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;

        file.write_all(&data).await?;
        file.flush().await?;
        
        self.current_offset += len;

        Ok((self.current_file_id, offset, len))
    }

    pub async fn read_proposal(&self, file_id: u64, offset: u64, len: u64) -> Result<Proposal> {
        let file_path = self.data_dir.join(format!("{:05}.log", file_id));
        let mut file = File::open(file_path).await?;
        
        file.seek(std::io::SeekFrom::Start(offset)).await?;
        let mut buffer = vec![0u8; len as usize];
        file.read_exact(&mut buffer).await?;
        
        let proposal: Proposal = serde_json::from_slice(&buffer)?;
        Ok(proposal)
    }

    pub async fn read_all(&self) -> Result<Vec<Proposal>> {
        let file_path = self.data_dir.join("00000.log");
        tracing::info!("Reading binlog from: {:?}", file_path);
        if !file_path.exists() {
            tracing::warn!("Binlog file not found at: {:?}", file_path);
            return Ok(Vec::new());
        }

        let mut file = File::open(&file_path).await?;
        let mut content = String::new();
        file.read_to_string(&mut content).await?;

        let mut proposals = Vec::new();
        let mut stream = serde_json::Deserializer::from_str(&content).into_iter::<Proposal>();

        while let Some(proposal) = stream.next() {
            match proposal {
                Ok(p) => proposals.push(p),
                Err(e) => tracing::error!("Error parsing proposal from binlog: {}", e),
            }
        }
        tracing::info!("Read {} proposals from binlog", proposals.len());

        Ok(proposals)
    }
}
