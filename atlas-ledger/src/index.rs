use rocksdb::{DB, Options};
use std::path::Path;
use atlas_common::error::Result;

use std::fmt;

pub struct Index {
    db: DB,
}

impl fmt::Debug for Index {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Index")
            .field("db", &"RocksDB")
            .finish()
    }
}

impl Index {
    pub fn new(data_dir: &str) -> Result<Self> {
        let path = Path::new(data_dir).join("index");
        let mut opts = Options::default();
        opts.create_if_missing(true);
        
        let db = DB::open(&opts, path).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        Ok(Self { db })
    }

    pub fn index_proposal(&mut self, id: &str, file_id: u64, offset: u64, len: u64) -> Result<()> {
        let key = format!("prop:{}", id);
        let value = serde_json::to_vec(&(file_id, offset, len))?;
        self.db.put(key, value).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(())
    }

    pub fn get_proposal_location(&self, id: &str) -> Result<Option<(u64, u64, u64)>> {
        let key = format!("prop:{}", id);
        if let Some(value) = self.db.get(key).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))? {
            let location: (u64, u64, u64) = serde_json::from_slice(&value)?;
            return Ok(Some(location));
        }
        Ok(None)
    }
}
