//! utils.rs
//!
//! Common types and helper implementations shared across AtlasDB.
//!
//! This module provides basic utilities such as unique node identifiers,
//! trait integrations, and conversion helpers.

pub mod node_id;
pub use node_id::NodeId;

pub mod security;
pub mod time;
