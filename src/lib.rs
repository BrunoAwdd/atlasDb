// lib.rs

pub mod cluster;
//pub mod ffi;
pub mod network;
pub mod utils;
pub mod peer_manager;
pub mod env;
pub mod builder;
pub mod cluster_proto {
    tonic::include_proto!("cluster");
}


// Reexporta os tipos principais para quem usar a lib

pub use cluster::{cluster::Cluster, node::Node};
pub use env::{
    consensus::{ConsensusEngine, Proposal, Vote, ConsensusResult},
    node::{Graph, Vertex, Edge},
    storage::{Storage, audit::{AuditData, save_audit, load_audit}}
};

pub use utils::NodeId;

pub use crate::network::adapter::NetworkAdapter;