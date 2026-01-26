use std::env;

/// Helper to parse simple arguments in the format --key value
pub fn get_arg_value<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|pos| args.get(pos + 1))
        .map(|s| s.as_str())
}

pub struct Args {
    pub p2p_listen_addr: String,
    pub dial_addr: Option<String>,
    pub grpc_port: String,
    pub config_path: String,
    pub keypair_path: String,
    pub test_auth: bool,
    pub args_vec: Vec<String>, // Keep original args for specific checks if needed
}

impl Args {
    pub fn parse() -> Self {
        let args: Vec<String> = env::args().collect();
        let p2p_listen_addr = get_arg_value(&args, "--listen").unwrap_or("/ip4/0.0.0.0/tcp/0").to_string();
        let dial_addr = get_arg_value(&args, "--dial").map(|s| s.to_string());
        let grpc_port = get_arg_value(&args, "--grpc-port").unwrap_or("50051").to_string();
        let config_path = get_arg_value(&args, "--config").unwrap_or("config.json").to_string();
        let keypair_path = get_arg_value(&args, "--keypair").unwrap_or("keys/keypair").to_string();
        let test_auth = args.contains(&"--test-auth".to_string());

        Self {
            p2p_listen_addr,
            dial_addr,
            grpc_port,
            config_path,
            keypair_path,
            test_auth,
            args_vec: args,
        }
    }
}
