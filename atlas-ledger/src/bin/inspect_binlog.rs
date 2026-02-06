use std::env;
use std::fs::File;
use std::io::BufReader;
use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let default_path = "example/node1/data/binlog/00000.log";
    let path = args.get(1).map(|s| s.as_str()).unwrap_or(default_path);

    eprintln!("🔍 Inspecting Binlog: {}", path);

    let file = File::open(path)?;
    let reader = BufReader::new(file);

    let stream = serde_json::Deserializer::from_reader(reader).into_iter::<Value>();
    
    let mut proposals = Vec::new();
    for result in stream {
        match result {
            Ok(mut val) => {
                // Try to parse 'content' field if it is a string
                if let Some(content_str) = val.get("content").and_then(|c| c.as_str()) {
                    if let Ok(parsed) = serde_json::from_str::<Value>(content_str) {
                        val["content"] = parsed;
                    }
                }
                proposals.push(val)
            },
            Err(e) => eprintln!("❌ Parse error: {}", e),
        }
    }

    eprintln!("✅ Parsed {} entries.", proposals.len());
    
    // Output valid JSON array
    println!("{}", serde_json::to_string_pretty(&proposals)?);

    Ok(())
}
