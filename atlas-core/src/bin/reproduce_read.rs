use std::fs::File;
use std::io::Read;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Proposal {
    id: String,
    // other fields are ignored for this test
}

fn main() {
    let path = "atlas-core/example/node1/data/binlog/00000.log";
    let mut file = File::open(path).expect("Failed to open file");
    let mut content = String::new();
    file.read_to_string(&mut content).expect("Failed to read file");

    println!("File content length: {}", content.len());

    let stream = serde_json::Deserializer::from_str(&content).into_iter::<Proposal>();
    let mut count = 0;
    for p in stream {
        match p {
            Ok(_) => count += 1,
            Err(e) => println!("Error: {}", e),
        }
    }
    println!("Parsed {} proposals", count);
}
