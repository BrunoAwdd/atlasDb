# AtlasDB Whitepaper

## The Account Event Chain (AEC) Protocol & Distributed Graph Database

**Version:** 0.1.0 (Draft)
**Date:** January 2025

---

## 1. Abstract

Blockchains have traditionally relied on monolithic global state trees (e.g., Merkle Patricia Tries) which, while secure, create significant bottlenecks in throughput and scalability. **AtlasDB** introduces a novel architecture based on the **Account Event Chain (AEC)** paradigm. By treating each account as an independent, linear chain of events and leveraging a strictly typed double-entry ledger, AtlasDB achieves massive parallelism, O(1) write complexity per account, and robust, audit-ready history without central indexers. This paper details the Atlas Protocol, the AEC storage model, and the asynchronous consensus mechanism that powers this distributed graph database.

## 2. Introduction

### 2.1 The Problem: The Monolithic Bottleneck

In traditional EVM-like architectures, every transaction competes for the same global state root. Validating a transaction requires updating a massive global tree, leading to sequential processing limitations. Furthermore, accessing historical data often requires expensive "archive nodes" or external indexers, as the chain itself focuses primarily on the _current_ state.

### 2.2 The Solution: AtlasDB & AEC

AtlasDB decouples **State** (Validation Cursor) from **History** (Truth).

- **State:** A lightweight, in-memory pointer (hash + nonce) maintained by validators for integrity checks.
- **History:** A persistent, distributed set of linear logs (files) specific to each account.

This separation allows transactions affecting disjoint sets of accounts to be processed in parallel. The consensus engine ensures eventual consistency across the network without blocking unrelated operations.

## 3. Architecture Overview

AtlasDB is built on a modular Rust architecture comprising three core layers:

1.  **The Network Layer (`atlas-p2p` & `atlas-node`)**: Handles peer discovery, gossip, and node identity.
2.  **The Consensus Layer (`atlas-consensus`)**: A pluggable, asynchronous consensus engine supporting weighted voting and slashing.
3.  **The Ledger Layer (`atlas-ledger`)**: The implementation of the Account Event Chain and the Chart of Accounts.

## 4. The Account Event Chain (AEC) Protocol

The AEC is the heart of AtlasDB. It redefines "on-chain data" from a global pool to a user-centric timeline.

### 4.1 "State is RAM, History is Disk"

Unlike traditional blockchains where the entire history governs the state calculation at every block, Atlas validators only keep the **Head State** in RAM:

- `last_transaction_hash`: The cryptographic link to the most recent event.
- `nonce`: A counter to prevent replay attacks.
- `balances`: Current asset holdings.

The actual transaction data is appended to an immutable "Shard" or "Segment" file on disk (`.bin`).

### 4.2 Chain Structure

Every transaction in AtlasDB is a "multichain" event. If `User A` sends tokens to `User B`, the transaction entry includes:

- A link to `User A`'s previous hash.
- A link to `User B`'s previous hash.

This explicitly links the history of interacting accounts, forming a **Directed Acyclic Graph (DAG)** of events rather than a single linear chain.

**Code Reference:**

```rust
// atlas-ledger/src/core/ledger/transaction_engine.rs
// Link to previous transaction hash for all involved accounts
for leg in &entry.legs {
    if let Some(prev_hash) = &account_state.last_transaction_hash {
        entry.prev_for_account.insert(leg.account.clone(), prev_hash.clone());
    }
}
```

### 4.3 High-Performance Storage

The AEC model writes data sequentially to disk. This leverages the operating system's file system caching and pre-fetching, enabling performance characteristics similar to streaming logs (Kafka) rather than random-access databases (B-Trees).

## 5. The Double-Entry Ledger System

AtlasDB enforces strict accounting principles at the protocol level. Money is never created or destroyed arbitrarily; it moves from one account to another.

### 5.1 Chart of Accounts

Every address in AtlasDB is classified into one of five root classes, enforced by the `AccountSchema`:

1.  **Ativo (Assets)** (`1.x`)
2.  **Passivo (Liabilities)** (`2.x` & Wallets `0x...`)
3.  **vault Liquido (Equity)** (`3.x`)
4.  **Resultado (Revenue/Expense)** (`4.x`)
5.  **Compensacao (Compensation)** (`5.x`)

**User Wallets (`0x...`)** are treated as **Liabilities** of the generic "Bank" (the Protocol). This reflects the reality that tokens are claims against the network.

### 5.2 Transaction Legs

A transaction is stable if and only if:
`Sum(Debits) == Sum(Credits)`

This guarantees invariant safety for the entire economy.

## 6. Consensus & Security

### 6.1 Asynchronous Weighted Voting

AtlasDB utilizes a proposal-based consensus engine. Nodes collect transactions into local "batches", form a Proposal, and broadcast it. Peers vote on proposals based on their **Stake Weight**.

**Code Reference:**

```rust
// atlas-consensus/src/consensus/evaluator.rs
// Weighted voting logic taking into account delegations
```

### 6.2 Slashing & Accountability

To prevent "Nothing-at-Stake" problems and Equivocation (Double Voting), the protocol implements automated slashing.
If a node is caught modifying history or voting for two conflicting proposals in the same view, a **Slashing Transaction** is automatically generated.

- **Validator Penalty:** A significant portion of their stake is burned (Moved to `vault:Slashing`).
- **Shared Risk:** Delegators share a % of the penalty, encouraging careful selection of validators.

## 7. Future Roadmap

1.  **Segmented Pruning:** Allow light nodes to only store "hot" segments of the AEC for specific accounts.
2.  **Zero-Knowledge Proofs:** Implement ZK-Rollups on top of AEC for privacy-preserving sub-chains.
3.  **Inter-Chain Communication:** Native bridges leveraging the explicit "Event Sourcing" nature of AEC to prove state to external chains without full block headers.

## 8. Conclusion

AtlasDB represents a shift from "World Computer" monoliths to "World Ledger" networks. By combining the rigorous safety of double-entry accounting with the scalability of the Account Event Chain architecture, it offers a pragmatic, high-performance foundation for the next generation of decentralized financial applications.
