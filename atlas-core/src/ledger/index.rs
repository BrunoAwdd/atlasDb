use rocksdb::{DB, Options};
use std::path::Path;
use crate::error::{AtlasError, Result};

#[derive(Debug)]
pub struct Index {
    db: DB,
}

impl Index {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        
        let db = DB::open(&opts, path)
            .map_err(|e| AtlasError::Other(format!("RocksDB open error: {}", e)))?;

        Ok(Self { db })
    }

    pub fn index_proposal(&mut self, id: &str, height: u64, file_id: u64, offset: u64, len: u64) -> Result<()> {
        // 1. ID -> Location
        let key_id = format!("prop:{}", id);
        let mut val = Vec::with_capacity(24);
        val.extend_from_slice(&file_id.to_be_bytes());
        val.extend_from_slice(&offset.to_be_bytes());
        val.extend_from_slice(&len.to_be_bytes());

        self.db.put(key_id.as_bytes(), &val)
            .map_err(|e| AtlasError::Other(format!("RocksDB put error (id): {}", e)))?;
        
        // 2. Height -> ID
        // Use "height:" prefix + 8 bytes BE for correct sorting
        let mut key_height = Vec::with_capacity(7 + 8);
        key_height.extend_from_slice(b"height:");
        key_height.extend_from_slice(&height.to_be_bytes());
        
        self.db.put(&key_height, id.as_bytes())
            .map_err(|e| AtlasError::Other(format!("RocksDB put error (height): {}", e)))?;

        Ok(())
    }

    pub fn get_proposal_location(&self, id: &str) -> Result<Option<(u64, u64, u64)>> {
        let key = format!("prop:{}", id);
        match self.db.get(key.as_bytes()) {
            Ok(Some(val)) => {
                if val.len() != 24 {
                    return Err(AtlasError::Other("Invalid index value length".to_string()));
                }
                let file_id = u64::from_be_bytes(val[0..8].try_into().unwrap());
                let offset = u64::from_be_bytes(val[8..16].try_into().unwrap());
                let len = u64::from_be_bytes(val[16..24].try_into().unwrap());
                Ok(Some((file_id, offset, len)))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(AtlasError::Other(format!("RocksDB get error: {}", e))),
        }
    }

    pub fn get_ids_after_height(&self, height: u64) -> Result<Vec<String>> {
        let mut ids = Vec::new();
        
        // Start iteration from height + 1
        let start_height = height + 1;
        let mut start_key = Vec::with_capacity(7 + 8);
        start_key.extend_from_slice(b"height:");
        start_key.extend_from_slice(&start_height.to_be_bytes());

        let mode = rocksdb::IteratorMode::From(&start_key, rocksdb::Direction::Forward);
        let iterator = self.db.iterator(mode);

        for item in iterator {
            let (key, value) = item.map_err(|e| AtlasError::Other(format!("RocksDB iter error: {}", e)))?;
            
            // Check if key starts with "height:"
            if !key.starts_with(b"height:") {
                break;
            }

            // Convert value (ID) to string
            let id = String::from_utf8(value.to_vec())
                .map_err(|e| AtlasError::Other(format!("Invalid UTF-8 ID: {}", e)))?;
            ids.push(id);
        }

        Ok(ids)
    }
}
