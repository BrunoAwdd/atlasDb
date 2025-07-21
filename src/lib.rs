// lib.rs
pub mod auth;
pub mod builder;
pub mod cluster;
pub mod cluster_proto {
    tonic::include_proto!("cluster");
}
pub mod env;
//pub mod ffi;
pub mod network;
pub mod peer_manager;
pub mod utils;

pub use cluster::{
    core::Cluster, 
    node::Node
};
pub use env::{
    consensus::{
        ConsensusEngine,
        ConsensusResult, 
        Vote
    },
    node::{
        Edge,
        Graph, 
        Vertex, 
    },
    proposal::Proposal,
    storage::{
        Storage, 
        audit::{
            AuditData,
            load_audit, 
            save_audit
        }
    }
};
pub use utils::NodeId;
pub use network::adapter::NetworkAdapter;