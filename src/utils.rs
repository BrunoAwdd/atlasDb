//! utils.rs
//!
//! Common types and helper implementations shared across AtlasDB.
//!
//! This module provides basic utilities such as unique node identifiers,
//! trait integrations, and conversion helpers.

use serde::{Serialize, Deserialize};

/// Unique identifier for a node in the distributed cluster.
///
/// `NodeId` is a lightweight wrapper around `String`, designed to:
/// - Ensure type safety across APIs
/// - Enable strong `HashMap`/`HashSet` keys
/// - Provide readable formatting and conversions
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub String);

impl std::fmt::Display for NodeId {
    /// Enables direct formatting via `{}` for logging and messages.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for NodeId {
    /// Converts from a string literal or slice to a `NodeId`.
    ///
    /// Example:
    /// ```rust
    /// use atlas_db::utils::NodeId;
    /// let id: NodeId = "node-A".into();
    /// ```
    fn from(s: &str) -> Self {
        NodeId(s.to_string())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashSet, HashMap};

    #[test]
    fn test_node_id_construction_and_display() {
        let id = NodeId("node-01".to_string());
        assert_eq!(id.0, "node-01");

        let formatted = format!("{}", id);
        assert_eq!(formatted, "node-01");
    }

    #[test]
    fn test_node_id_from_str() {
        let id: NodeId = "peer-A".into();
        assert_eq!(id.0, "peer-A".to_string());
    }

    #[test]
    fn test_node_id_equality() {
        let a = NodeId("n".into());
        let b = NodeId("n".into());
        let c = NodeId("x".into());

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn test_node_id_hashing() {
        let mut map = HashMap::new();
        map.insert(NodeId("n1".into()), "active");
        map.insert(NodeId("n2".into()), "idle");

        assert_eq!(map.get(&NodeId("n1".into())), Some(&"active"));
        assert_eq!(map.get(&NodeId("n2".into())), Some(&"idle"));

        let set: HashSet<NodeId> = map.keys().cloned().collect();
        assert!(set.contains(&NodeId("n1".into())));
    }
}
