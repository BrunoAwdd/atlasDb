// src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AtlasError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Consensus error: {0}")]
    Consensus(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Invalid config: {0}")]
    Config(String),

    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other: {0}")]
    Other(String),
}
