#[derive(Clone, Debug)]
pub struct P2pConfig {
    pub listen_multiaddrs: Vec<String>, // e.g. ["/ip4/0.0.0.0/tcp/4001"]
    pub bootstrap: Vec<String>,         // e.g. ["/ip4/.../p2p/<peerid>"]
    pub enable_mdns: bool,
    pub enable_kademlia: bool,
    pub keypair_path: String,
}
