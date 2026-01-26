# RFC 001: Secure State Transfer via Hash Chains

## Status

Draft

## Summary

This RFC proposes enhancing the State Transfer (synchronization) mechanism by introducing hash-based verification. Instead of relying solely on block height, nodes will exchange and verify the cryptographic hash of the latest block (and potentially a chain of hashes) to ensure data integrity and prevent synchronization with malicious nodes offering a divergent history.

## Motivation

Currently, AtlasDB nodes synchronize based on `height`. If a node is at height 10 and peers are at height 15, it requests proposals 11-15.
**Vulnerability:** A malicious node could provide a fake history for blocks 11-15. As long as the format is valid, the requesting node might accept it, leading to a "split brain" or corrupted state where the node's truth diverges from the honest majority.

## Detailed Design

### 1. Proposal Hash Chain

Each `Proposal` already contains a `prev_hash` field. We must enforce that:
`Proposal[N].prev_hash == Hash(Proposal[N-1])`

### 2. Sync Handshake

When a node requests a sync, it should include:

- Its current Height ($H$)
- The Hash of its last block ($Hash_H$)

The peer receiving the request must verify:

- Does my local block at height $H$ have the same hash as $Hash_H$?
  - **YES:** The requester is on the same chain. Send blocks $H+1$ to $Tip$.
  - **NO:** The requester is on a fork or corrupt chain. Reject sync or initiate a "Fork Recovery" process (rollback).

### 3. Response Verification

When receiving the bundle of proposals, the requester must verify:

- `Proposal[H+1].prev_hash == Hash(Proposal[H])` (My last block)
- `Proposal[H+2].prev_hash == Hash(Proposal[H+1])`
- ...and so on.

### 4. Merkle Tree (Future Work)

For larger state transfers, we can implement a Merkle Tree where the state root is signed by the quorum. This allows verifying a snapshot without replaying the entire history.

## Drawbacks

- **Computation:** Calculating hashes for every sync request adds slight overhead (negligible for SHA-256).
- **Complexity:** Handling forks (where a node must discard its local history to follow the majority) is complex to implement safely.

## Alternatives

- **Checkpointing:** Periodically (e.g., every 100 blocks) create a "Checkpoint" signed by 2/3+ of the cluster. Nodes only need to sync from the last checkpoint.
