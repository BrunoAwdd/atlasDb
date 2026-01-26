//! consensus.rs
//!
//! Asynchronous consensus simulation engine with probabilistic voting and quorum evaluation.
//!
//! This module simulates the core logic of a distributed consensus protocol,
//! where nodes vote independently on proposals and quorum is used to determine acceptance.
//!
//! The engine is deliberately asynchronous, failure-tolerant, and latency-aware,
//! serving as a conceptual foundation rather than a production-grade implementation.


mod engine;
pub mod evaluator;
mod pool;
mod registry;

pub use engine::ConsensusEngine;
