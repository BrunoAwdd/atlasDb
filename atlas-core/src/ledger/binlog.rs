use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use crate::env::proposal::Proposal;
use crate::error::{AtlasError, Result};

#[derive(Debug)]
pub struct Binlog {
    dir: PathBuf,
    current_file: File,
    current_file_id: u64,
    current_offset: u64,
}

impl Binlog {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let dir = path.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;

        // Simple implementation: always start/append to 00001.log for now
        // In production, we'd scan for the last log file.
        let current_file_id = 1;
        let file_path = dir.join(format!("{:05}.log", current_file_id));
        
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&file_path)?;

        let current_offset = file.seek(SeekFrom::End(0))?;

        Ok(Self {
            dir,
            current_file: file,
            current_file_id,
            current_offset,
        })
    }

    pub fn append_proposal(&mut self, proposal: &Proposal) -> Result<(u64, u64, u64)> {
        let bytes = bincode::serialize(proposal)
            .map_err(|e| AtlasError::Other(format!("Serialize error: {}", e)))?;
        
        let len = bytes.len() as u64;
        self.current_file.write_all(&bytes)?;
        self.current_file.flush()?;

        let offset = self.current_offset;
        self.current_offset += len;

        Ok((self.current_file_id, offset, len))
    }

    pub fn read_proposal(&self, file_id: u64, offset: u64, len: u64) -> Result<Proposal> {
        let file_path = self.dir.join(format!("{:05}.log", file_id));
        let mut file = File::open(file_path)?;
        
        file.seek(SeekFrom::Start(offset))?;
        let mut buf = vec![0u8; len as usize];
        file.read_exact(&mut buf)?;

        let proposal = bincode::deserialize(&buf)
            .map_err(|e| AtlasError::Other(format!("Deserialize error: {}", e)))?;

        Ok(proposal)
    }
}
