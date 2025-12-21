pub mod adapter;
pub mod behaviour;
pub mod codec;
pub mod config;
pub mod error;
pub mod events;
pub mod in_memory;
pub mod key_manager;
pub mod message;
pub mod peer_manager;
pub mod ports;
pub mod protocol;
pub mod traits;
pub mod utils;

// Re-export common types if needed
pub use peer_manager::PeerManager;
pub use config::P2pConfig;
pub use adapter::Libp2pAdapter;
