use atlas_db::rpc::client::{submit_proposal, get_status};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("  {} <node_address> <proposal_content>", args[0]);
        eprintln!("  {} status <node_address>", args[0]);
        return Ok(());
    }

    if args[1] == "status" {
        if args.len() < 3 {
            eprintln!("Usage: {} status <node_address>", args[0]);
            return Ok(());
        }
        let node_address = args[2].clone();
        match get_status(node_address).await {
            Ok(status) => {
                println!("Node ID: {}", status.node_id);
                println!("Leader ID: {}", status.leader_id);
                println!("Height: {}", status.height);
                println!("View: {}", status.view);
            }
            Err(e) => {
                eprintln!("Error getting status: {}", e);
            }
        }
    } else {
        if args.len() < 3 {
            eprintln!("Usage: {} <node_address> <proposal_content>", args[0]);
            return Ok(());
        }
        let node_addresses = vec![args[1].clone()];
        let content = args[2].clone();

        match submit_proposal(node_addresses, content).await {
            Ok(reply) => {
                println!("Proposal submitted successfully: {}", reply.message);
                println!("Proposal ID: {}", reply.proposal_id);
            }
            Err(e) => {
                eprintln!("Error submitting proposal: {}", e);
            }
        }
    }

    Ok(())
}
