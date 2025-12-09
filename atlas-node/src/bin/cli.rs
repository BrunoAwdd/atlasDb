use atlas_node::rpc::client::submit_proposal;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

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

    Ok(())
}
