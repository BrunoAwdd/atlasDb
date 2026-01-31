use redb::{Database, TableDefinition, ReadableTable};
use std::path::Path;
use atlas_common::error::Result;
use std::fmt;

const PROPOSALS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("proposals");
const TX_HASHES_TABLE: TableDefinition<&str, &str> = TableDefinition::new("tx_hashes");

pub struct Index {
    db: Database,
}

impl fmt::Debug for Index {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Index")
            .field("db", &"Redb")
            .finish()
    }
}

impl Index {
    pub fn new(data_dir: &str) -> Result<Self> {
        // Redb is a single file, typically not a directory like RocksDB.
        // We ensure data_dir exists.
        std::fs::create_dir_all(data_dir)?;
        let path = Path::new(data_dir).join("index.redb");
        
        let db = Database::create(path).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        // Initialize tables
        let write_txn = db.begin_write().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        {
            let _table = write_txn.open_table(PROPOSALS_TABLE).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let _table2 = write_txn.open_table(TX_HASHES_TABLE).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }
        write_txn.commit().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        Ok(Self { db })
    }

    pub fn index_proposal(&mut self, id: &str, hash: &str, file_id: u64, offset: u64, len: u64) -> Result<()> {
        let value = serde_json::to_vec(&(file_id, offset, len))?;
        
        let write_txn = self.db.begin_write().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        {
            let mut table = write_txn.open_table(PROPOSALS_TABLE).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            
            // Check existence (Idempotency)
            if table.get(id).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?.is_some() {
                 return Ok(()); // Already indexed
            }

            table.insert(id, value.as_slice()).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            let mut table_hashes = write_txn.open_table(TX_HASHES_TABLE).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            table_hashes.insert(hash, id).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        }
        write_txn.commit().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        Ok(())
    }

    pub fn get_proposal_location(&self, id: &str) -> Result<Option<(u64, u64, u64)>> {
        let read_txn = self.db.begin_read().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let table = read_txn.open_table(PROPOSALS_TABLE).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        if let Some(value) = table.get(id).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))? {
            let location: (u64, u64, u64) = serde_json::from_slice(value.value())?;
            return Ok(Some(location));
        }
        Ok(None)
    }

    pub fn exists_tx(&self, hash: &str) -> Result<bool> {
        let read_txn = self.db.begin_read().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let table = read_txn.open_table(TX_HASHES_TABLE).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        let result = match table.get(hash) {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e).into()),
        };
        result
    }
}
