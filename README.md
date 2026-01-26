![Rust](https://img.shields.io/badge/Rust-Async_Consensus-orange?style=flat&logo=rust)

# 🌐 AtlasDB

> A distributed, peer-to-peer graph database experiment with asynchronous consensus and modular architecture. Written in Rust for maximum performance, safety, and expressive power.

---

## 🚀 Vision

**AtlasDB** is a technical exploration into the future of distributed databases — focused on graph structures and node-to-node consensus without any centralized coordinator.

Inspired by concepts like Raft and eventual consistency, AtlasDB embraces **network latency, node variability, and failure tolerance** as fundamental design elements. It aims to simulate how graph data could be shared, replicated, and validated in truly decentralized environments.

This project is not production-ready — it’s **a proof of architecture**, crafted to provoke, demonstrate, and inspire.

---

## 🧱 Architecture Overview

| Module         | Description                                                               |
| -------------- | ------------------------------------------------------------------------- |
| `node.rs`      | Core graph structure: vertices, edges, metadata, and traversal methods    |
| `cluster.rs`   | Simulated peer-to-peer cluster with nodes, inboxes, and heartbeat logic   |
| `consensus.rs` | Asynchronous consensus engine with quorum voting and JSON-based proposals |
| `storage.rs`   | Lightweight in-memory ledger of proposals, votes, and consensus results   |
| `utils.rs`     | Shared types and helpers (e.g., `NodeId`)                                 |

---

## ⚙️ Features

- 🧠 **Graph-oriented**: Built from the ground up to model interconnected data
- 🤝 **Decentralized voting**: Each node votes independently on proposals
- ⏱️ **Asynchronous simulation**: Includes simulated latency and heartbeat cycles
- 📦 **Proposal-driven updates**: Only approved consensus proposals mutate the graph
- 🧾 **Audit-ready**: All actions logged and stored via `Storage`

---

## 🧪 Example Usage

```bash
cargo run --example start_cluster
```

This will:

1. Start a simulated 5-node cluster
2. Propose a graph modification (`add_edge`)
3. Have each node vote independently (with 90% chance of approval)
4. Apply the change to the graph **only if quorum is reached**
5. Print the final state and voting logs

---

## 📌 Why It Exists

> Building databases is hard.  
> Building distributed consensus is harder.  
> AtlasDB exists to explore what happens when we try to build **both**, from scratch — intentionally, imperfectly, and in public.

---

## 🔄 How It Works

1. Nodes form a simulated cluster with local graph replicas.
2. A node submits a mutation proposal (e.g., add edge).
3. All peers vote independently. If quorum is reached, the graph is updated.

## 🧪 Example Output

```bash
📤 Submitting proposal...
🕒 Simulating voting...
🗳️ Proposal [prop-1] received 4/5 YES votes — APPROVED ✅
✅ Edge added to graph: [A] --visits--> [B]

📌 Final graph state:
🔍 Vertices:
- [A] Person
- [B] Place
🔗 Edges:
> [A] --visits--> [B]
```

## 🛠️ Next Steps (Optional)

- Implement real message-passing between `ClusterNode`s
- Add proposal validation logic
- Extend graph with versioning or hashing
- Export to Graphviz or Neo4j-compatible format
- Replace `Storage` with real persistence

---

## 🤝 Contributing

This project welcomes exploratory ideas and improvements. PRs and discussions are open!

## 📖 License

MIT — use it, fork it, learn from it.

6wR8xmn5wo
