
pub struct Args {
    pub p2p_listen_addr: String,
    pub dial_addr: Option<String>,
    pub grpc_port: String,
    pub config_path: String,
    pub keypair_path: String,
}

impl Args {
    pub fn parse() -> Self {
        let args: Vec<String> = std::env::args().collect();
        
        // Provide defaults here, but allow overrides
        Self {
            p2p_listen_addr: get_arg_value(&args, "--listen").unwrap_or("/ip4/0.0.0.0/tcp/0").to_string(),
            dial_addr: get_arg_value(&args, "--dial").map(|s| s.to_string()),
            grpc_port: get_arg_value(&args, "--grpc-port").unwrap_or("50051").to_string(),
            config_path: get_arg_value(&args, "--config").unwrap_or("config.json").to_string(),
            keypair_path: get_arg_value(&args, "--keypair").unwrap_or("keys/keypair").to_string(),
        }
    }
}

fn get_arg_value<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter()
        .position(|arg| arg == key)
        .and_then(|pos| args.get(pos + 1))
        .map(|s| s.as_str())
}
