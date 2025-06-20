//! node.rs
//!
//! Defines the foundational structure for graph modeling: vertices, edges, and the graph itself.
//!
//! This module is designed for decentralized environments, with extensibility
//! in mind — including future support for graph versioning, change tracking,
//! and structural diffs.

use std::collections::HashMap;

/// A graph vertex, uniquely identified and semantically labeled.
///
/// Each vertex may contain arbitrary key-value metadata (`properties`).
#[derive(Debug, Clone)]
pub struct Vertex {
    /// Unique identifier of the vertex.
    pub id: String,

    /// Semantic label for classification (e.g., "Person", "File", "Event").
    pub label: String,

    /// Arbitrary property map for contextual data (e.g., {"age": "42"}).
    pub properties: HashMap<String, String>,
}

impl Vertex {
    /// Constructs a new vertex with the given ID and label.
    pub fn new(id: &str, label: &str) -> Self {
        Vertex {
            id: id.to_string(),
            label: label.to_string(),
            properties: HashMap::new(),
        }
    }

    /// Adds a single key-value property to the vertex (fluent-style).
    pub fn with_property(mut self, key: &str, value: &str) -> Self {
        self.properties.insert(key.to_string(), value.to_string());
        self
    }
}

/// Represents a directed edge between two vertices.
///
/// Edges are labeled and directionally link two vertex IDs.
#[derive(Debug, Clone)]
pub struct Edge {
    /// Source vertex ID.
    pub from: String,

    /// Destination vertex ID.
    pub to: String,

    /// Relationship label (e.g., "likes", "follows", "owns").
    pub label: String,
}

impl Edge {
    /// Constructs a new directed edge from `from` → `to` with a label.
    pub fn new(from: &str, to: &str, label: &str) -> Self {
        Edge {
            from: from.to_string(),
            to: to.to_string(),
            label: label.to_string(),
        }
    }
}

/// Represents the local graph state of a node.
///
/// Tracks all known vertices and edges in a directed graph model.
#[derive(Debug, Default)]
pub struct Graph {
    /// Map of vertex ID → vertex data.
    pub vertices: HashMap<String, Vertex>,

    /// List of directed edges in the graph.
    pub edges: Vec<Edge>,
}

impl Graph {
    /// Creates an empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a new vertex into the graph.
    ///
    /// If a vertex with the same ID already exists, it will be replaced.
    pub fn add_vertex(&mut self, vertex: Vertex) {
        self.vertices.insert(vertex.id.clone(), vertex);
    }

    /// Adds a directed edge to the graph.
    ///
    /// Assumes that `from` and `to` vertices are already present.
    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
    }

    /// Returns all neighbor vertices directly reachable from the given vertex ID.
    ///
    /// This only considers outgoing edges (`from`).
    pub fn neighbors_of(&self, id: &str) -> Vec<&Vertex> {
        self.edges
            .iter()
            .filter(|e| e.from == id)
            .filter_map(|e| self.vertices.get(&e.to))
            .collect()
    }

    /// Prints a simple representation of the graph's vertices and edges.
    pub fn print_graph(&self) {
        println!("🔍 Vertices:");
        for v in self.vertices.values() {
            println!("- [{}] {}", v.id, v.label);
        }

        println!("🔗 Edges:");
        for e in &self.edges {
            println!("> [{}] --{}--> [{}]", e.from, e.label, e.to);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_creation_and_properties() {
        let v = Vertex::new("v1", "Person")
            .with_property("name", "Alice")
            .with_property("age", "30");

        assert_eq!(v.id, "v1");
        assert_eq!(v.label, "Person");
        assert_eq!(v.properties.get("name"), Some(&"Alice".to_string()));
        assert_eq!(v.properties.get("age"), Some(&"30".to_string()));
    }

    #[test]
    fn test_edge_creation() {
        let e = Edge::new("v1", "v2", "knows");
        assert_eq!(e.from, "v1");
        assert_eq!(e.to, "v2");
        assert_eq!(e.label, "knows");
    }

    #[test]
    fn test_add_vertex_and_edge_to_graph() {
        let mut g = Graph::new();
        let v1 = Vertex::new("v1", "Person");
        let v2 = Vertex::new("v2", "Person");
        let e = Edge::new("v1", "v2", "knows");

        g.add_vertex(v1.clone());
        g.add_vertex(v2.clone());
        g.add_edge(e.clone());

        assert_eq!(g.vertices.len(), 2);
        assert_eq!(g.edges.len(), 1);
        assert!(g.vertices.contains_key("v1"));
        assert!(g.vertices.contains_key("v2"));
        assert_eq!(g.edges[0].from, "v1");
        assert_eq!(g.edges[0].to, "v2");
    }

    #[test]
    fn test_neighbors_of_vertex() {
        let mut g = Graph::new();
        g.add_vertex(Vertex::new("a", "City"));
        g.add_vertex(Vertex::new("b", "City"));
        g.add_vertex(Vertex::new("c", "City"));

        g.add_edge(Edge::new("a", "b", "road"));
        g.add_edge(Edge::new("a", "c", "rail"));

        let neighbors = g.neighbors_of("a");

        let neighbor_ids: Vec<&String> = neighbors.iter().map(|v| &v.id).collect();
        assert!(neighbor_ids.contains(&&"b".to_string()));
        assert!(neighbor_ids.contains(&&"c".to_string()));
        assert_eq!(neighbor_ids.len(), 2);
    }

    #[test]
    fn test_replace_existing_vertex_by_id() {
        let mut g = Graph::new();

        let v1 = Vertex::new("x", "File").with_property("name", "file1.txt");
        let v2 = Vertex::new("x", "File").with_property("name", "file2.txt");

        g.add_vertex(v1);
        assert_eq!(g.vertices["x"].properties["name"], "file1.txt");

        g.add_vertex(v2); // replaces existing
        assert_eq!(g.vertices["x"].properties["name"], "file2.txt");
        assert_eq!(g.vertices.len(), 1); // still only one vertex
    }

    #[test]
    fn test_neighbors_of_returns_empty_when_no_edges() {
        let mut g = Graph::new();
        g.add_vertex(Vertex::new("solo", "Node"));

        let neighbors = g.neighbors_of("solo");
        assert!(neighbors.is_empty());
    }
}
