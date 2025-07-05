# 🧭 Development Roadmap: Abstract Network for AtlasEnv

> Modular architecture for decentralized communication, focusing on pluggability, distributed validation, and peer control.

---

## ✅ Phase 1 – Core Structure

### 📌 Goal

Create a generic `Network` interface to support multiple implementations (in-memory, TCP, libp2p, etc.).

### 📋 Tasks

- [x] Define `ClusterMessage` enum with:
  - Signed proposal
  - Signed vote
- [x] Define `NetworkError` enum
- [x] Create `trait Network` with:
  - `send_to`
  - `broadcast`
  - `connected_peers`
  - `set_message_handler`
- [x] Make the **network injectable** (via trait object `Arc<dyn Network>`)

---

## 🧪 Phase 2 – Simulated Implementation (InMemoryNetwork)

### 📌 Goal

Provide an in-memory network for simulation and local testing.

### 📋 Tasks

- [ ] Create `InMemoryNetwork` struct
- [ ] Manage `peers: HashMap<NodeId, Sender<ClusterMessage>>`
- [ ] Implement `send_to` and `broadcast` using `tokio::sync::mpsc` channels
- [ ] Implement `set_message_handler` with `Fn(ClusterMessage)` callback

---

## 🔑 Phase 3 – Decoupled Authentication

### 📌 Goal

Allow swapping the message authentication/signature mechanism.

### 📋 Tasks

- [ ] Make **authentication system injectable** (e.g. `trait Authenticator`)
- [ ] Separate `sign()` and `verify()` for proposals and votes
- [ ] Support multiple signature schemes (ed25519, secp256k1, etc.)

---

## 🔁 Phase 4 – Callback Integration into AtlasEnv

### 📌 Goal

Allow `AtlasEnv` to react to received consensus messages.

### 📋 Tasks

- [ ] Add channel or closure in `NetworkAdapter` for `on_message(msg)`
- [ ] Integrate `set_message_handler` with `ConsensusEngine` and `Storage`
- [ ] Allow Atlas to decide the message's destination (vote, ignore, etc.)

---

## 🌱 Phase 5 – Root Proposals (`parent = None`)

### 📌 Goal

Enable proposals without a parent (root proposals), allowing graph bootstrapping.

### 📋 Tasks

- [ ] Update `Proposal` model to support `parent: Option<String>`
- [ ] Validate root proposals as consensus starting points
- [ ] Create test for consensus bootstrapping without ancestry

---

## 🌐 Phase 6 – Peer Management

### 📌 Goal

Enable each node to dynamically maintain ~30 peers.

### 📋 Tasks

- [ ] Add `PeerManager` module
- [ ] Implement peer discovery by propagation
- [ ] Implement removal of slow peers
- [ ] Enforce peer limit and overflow queue

---

## 🗳️ Phase 7 – Asynchronous Consensus with Quorum

### 📌 Goal

Validate proposals asynchronously with a minimum quorum (e.g. 20 votes).

### 📋 Tasks

- [ ] Node A sends to its 30 peers
- [ ] A stores votes until reaching quorum
- [ ] After quorum, proposal is published
- [ ] Expired messages (by timestamp) are ignored

---

## 🔌 Phase 8 – Real Network Adapters

### 📌 Goal

Support multiple real-world network adapters.

### 📋 Tasks

- [ ] `WebSocketAdapter` with `tokio-tungstenite`
- [ ] `Libp2pAdapter` with automatic peer discovery
- [ ] Add pluggability via `AtlasEnv::new(..., Box<dyn Network>)`

---

## 🧹 Phase 9 – Optimizations & Resilience

### 📌 Goal

Make the network robust and secure against byzantine failures.

### 📋 Tasks

- [ ] Signature verification (using injected Auth)
- [ ] Message deduplication
- [ ] Network logging and metrics
- [ ] Support for node re-entry after disconnection

---

## 🧩 Phase 10 – Extras

- [ ] Optional message compression
- [ ] Local persistence of peer list
- [ ] Future support for WebAssembly and WebRTC
