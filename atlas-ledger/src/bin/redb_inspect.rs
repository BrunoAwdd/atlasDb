use redb::{Database, TableDefinition, ReadableTable};
use std::env;
use std::path::Path;

const PROPOSALS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("proposals");
const TX_HASHES_TABLE: TableDefinition<&str, &str> = TableDefinition::new("tx_hashes");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <data_dir_or_redb_file>", args[0]);
        std::process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let db_path = if input_path.is_dir() {
        input_path.join("index.redb")
    } else {
        input_path.to_path_buf()
    };

    println!("📂 Opening Redb: {:?}", db_path);
    let db = Database::create(&db_path)?; // Create opens existing if exists

    let read_txn = db.begin_read()?;

    // List Proposals
    println!("\n--- PROPOSALS TABLE ---");
    match read_txn.open_table(PROPOSALS_TABLE) {
        Ok(table) => {
            let mut count = 0;
            for result in table.iter()? {
                let (key, value) = result?;
                let location: (u64, u64, u64) = serde_json::from_slice(value.value())?;
                println!("ID: {:<20} | Location: File={}, Offset={}, Len={}", key.value(), location.0, location.1, location.2);
                count += 1;
            }
            println!("Total Proposals: {}", count);
        },
        Err(e) => println!("Error opening table: {}", e),
    }

    // List Tx Hashes
    println!("\n--- TX HASHES TABLE ---");
    match read_txn.open_table(TX_HASHES_TABLE) {
        Ok(table) => {
            let mut count = 0;
            for result in table.iter()? {
                let (hash, proposal_id) = result?;
                println!("Hash: {}... -> Prop: {}", &hash.value()[..12], proposal_id.value());
                count += 1;
            }
            println!("Total Txs: {}", count);
        },
        Err(e) => println!("Error opening table: {}", e),
    }

    Ok(())
}
