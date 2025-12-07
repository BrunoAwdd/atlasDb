pub mod adapter;
pub mod config;
pub mod events;
pub mod traits;
pub mod peer_manager;
pub mod ports;
pub mod key_manager;
pub mod in_memory; // If needed, or remove

// Re-export common types if needed
pub use peer_manager::PeerManager;
pub use config::P2pConfig;
pub use adapter::Libp2pAdapter;
