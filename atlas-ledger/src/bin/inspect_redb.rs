use redb::{Database, ReadableTable, TableDefinition, TableHandle};
use std::env;
use std::path::Path;

// Must match the definition in atlas-ledger/src/core/runtime/index.rs
const PROPOSALS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("proposals");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let default_path = "data/index.redb";
    
    // Allow passing path as argument, otherwise use default
    let path_str = if args.len() > 1 {
        &args[1]
    } else {
        // Try to find the file in common locations if not provided
        if Path::new(default_path).exists() {
            default_path
        } else if Path::new("index.redb").exists() {
            "index.redb"
        } else {
             println!("Usage: inspect_redb <path_to_index.redb>");
             println!("No path provided and default '{}' not found.", default_path);
             println!("Please provide the path to index.redb as an argument.");
             return Ok(());
        }
    };

    let path = Path::new(path_str);
    if !path.exists() {
        eprintln!("Error: File not found at {:?}", path);
        std::process::exit(1);
    }

    println!("Opening database at {:?}", path);
    // Open the database in read-only mode
    let db = Database::open(path)?;
    let read_txn = db.begin_read()?;
    
    println!("Opening 'proposals' table...");
    match read_txn.open_table(PROPOSALS_TABLE) {
        Ok(table) => {
            let mut count = 0;
            println!("{:<64} | {:<10} | {:<10} | {:<10}", "Proposal ID", "File ID", "Offset", "Length");
            println!("{:-<64}-+-{:-<10}-+-{:-<10}-+-{:-<10}", "", "", "", "");

            for result in table.iter()? {
                let (key, value) = result?;
                // Deserialize the value: (file_id, offset, len)
                // Note: The value in redb is a slice, we need `value.value()` to get &[u8]
                let location: (u64, u64, u64) = serde_json::from_slice(value.value())
                    .map_err(|e| format!("Failed to deserialize value for key {}: {}", key.value(), e))?;
                
                println!("{:<64} | {:<10} | {:<10} | {:<10}", 
                    key.value(), 
                    location.0, 
                    location.1, 
                    location.2
                );
                count += 1;
            }
            println!("\nTotal proposals indexed: {}", count);
        }
        Err(e) => {
            eprintln!("Could not open 'proposals' table: {}. The table might not exist yet.", e);
            println!("Listing all available tables:");
            let tables = read_txn.list_tables()?;
            for t in tables {
                println!(" - {}", t.name());
            }
        }
    }

    Ok(())
}
