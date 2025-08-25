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

- [x] Create `InMemoryNetwork` struct
- [ ] Manage `peers: HashMap<NodeId, Sender<ClusterMessage>>`
- [x] Implement `send_to` and `broadcast` using `tokio::sync::mpsc` channels
- [x] Implement `set_message_handler` with `Fn(ClusterMessage)` callback

---

## 🔑 Phase 3 – Decoupled Authentication

### 📌 Goal

Allow swapping the message authentication/signature mechanism.

### 📋 Tasks

- [x] Make **authentication system injectable** (e.g. `trait Authenticator`)
- [x] Separate `sign()` and `verify()` for proposals and votes
- [x] Support multiple signature schemes (ed25519, secp256k1, etc.)

---

## 🔁 Phase 4 – Callback Integration into AtlasEnv

### 📌 Goal

Allow `AtlasEnv` to react to received consensus messages.

### 📋 Tasks

- [x] Add channel or closure in `NetworkAdapter` for `on_message(msg)`
- [x] Integrate `set_message_handler` with `ConsensusEngine` and `Storage`
- [x] Allow Atlas to decide the message's destination (vote, ignore, etc.)

---

## 🌱 Phase 5 – Root Proposals (`parent = None`)

### 📌 Goal

Enable proposals without a parent (root proposals), allowing graph bootstrapping.

### 📋 Tasks

- [x] Update `Proposal` model to support `parent: Option<String>`
- [x] Validate root proposals as consensus starting points
- [ ] Create test for consensus bootstrapping without ancestry

---

## 🌐 Phase 6 – Peer Management

### 📌 Goal

Enable each node to dynamically maintain ~30 peers.

### 📋 Tasks

- [x] Add `PeerManager` module
- [x] Implement peer discovery by propagation
- [x] Implement removal of slow peers
- [x] Enforce peer limit and overflow queue

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

- [x] Signature verification (using injected Auth)
- [ ] Message deduplication
- [ ] Network logging and metrics
- [ ] Support for node re-entry after disconnection

---

## 🧩 Phase 10 – Extras

- [ ] Optional message compression
- [ ] Local persistence of peer list
- [ ] Future support for WebAssembly and WebRTC

---

# AtlasDB (Fiducial) — Checklist de Implementação

## P0 — Fundação e Segurança do Núcleo

- [ ] **Erros tipados (thiserror) unificados**
  - Criar `AtlasError` + `type Result<T> = std::result::Result<T, AtlasError>;`
  - Migrar `Result<_, String>` → `Result<_, AtlasError>` em `cluster/*`, `jobs/*`, `runtime/*`, `env/*`.
- [ ] **Política de quórum**
  - Implementar `QuorumPolicy { fraction ≥ 0.5, min_voters }` e usar em agregação de votos/acks.
  - Parametrizar via config.
- [ ] **Observabilidade mínima**
  - Trocar `println!` → `tracing::{info,warn,error,debug}`.
  - `main.rs`: configurar `tracing_subscriber` com `EnvFilter` (`RUST_LOG=atlas=info,tokio=warn`).
  - Usar `#[instrument]` nos handlers gRPC e hot paths (bus worker, broadcast/handle).

---

## P1 — I/O Confiável e Reprodutível

- [ ] **Persistência (KV mínima)**
  - Backend (ex.: `sled`): `put/get`.
  - Persistir peers, audit de mensagens (proposals/votes), e metadados essenciais.
  - Recarregar no boot (estado base do cluster).
- [ ] **mTLS na camada gRPC**
  - Server/Client com `Identity` + `client_ca_root`.
  - Configurar paths no `config.json`.
  - (Opcional) mapear identidade do peer a `NodeId`.

---

## P2 — Consenso Completo (entrada → decisão → saída)

- [ ] **Mempool dentro do consenso (entrada)**
  - Trait `Mempool` (add/select/mark_included).
  - Integrar no `ConsensusEngine`/proposer para montar propostas/blocos.
- [ ] **Ledger dentro do consenso (saída)**
  - Trait `Ledger` (apply_block/apply_decision, replay).
  - Ao atingir quórum: aplicar no ledger, registrar no KV, limpar mempool.
  - Garantir idempotência (replay após restart).
- [ ] **Fechamento de consenso → commit**
  - `HandleVote/Proposal`: quando `grants >= quorum_required`, fechar decisão e acionar `ledger.apply_*`.

---

## P3 — Runtime & Jobs Robustos

- [ ] **Runtime/Builder consolidado (sem anyhow)**
  - `build_runtime()` retorna `Result<_, AtlasError>`.
  - `CommandBus::new(arc.clone(), ..)` (sem `&`), timeout por `cmd.timeout()`.
  - Agendar `BroadcastHeartbeat` com jitter no `Scheduler`.
- [ ] **Melhorias no CommandBus/Scheduler**
  - Logs estruturados por `job_id`, `cmd`, `status`, `dur`.
  - (Opcional) Retry/backoff para `NoQuorum` no Bus ou Scheduler.

---

## P4 — Configuração & Validação

- [ ] **Config completa**
  - `quorum_min_voters`, `quorum_fraction`.
  - `tls: { cert_path, key_path, ca_path }`.
  - `storage_path` (KV).
  - Validação: `fraction ≥ 0.5`, caminhos existentes.

---

## P5 — Telemetria (depois do básico)

- [ ] **Métricas**
  - Contadores: `heartbeats_sent/recv`, `jobs_completed/failed/timeout`, `votes_granted`, `quorum_required`, `mempool_size`.
  - (Opcional) `/metrics` Prometheus.

---

## Sequência Sugerida

1. **P0** (Erros + Quórum + Logs)
2. **P1** (KV + mTLS)
3. **P2** (Mempool + Ledger + Commit Path)
4. **P3** (Runtime/Jobs refinado)
5. **P4** (Config final)
6. **P5** (Métricas)

---

# ✅ Checklist Fiducial vs PIX

## Core

- [ ] **Definir estados de transação**
  - `accepted_optimistic` → `finalized` → `checkpointed`
  - (e opcional: `reverted`)
- [ ] **Implementar dois modos de confirmação (M0/M1)**
  - M0: validação local rápida (optimistic)
  - M1: consenso com quorum (finality)
- [ ] **Limites de M0**
  - Valor máximo
  - Frequência por conta
  - Score/reputação

---

## Execução / Performance

- [ ] **Paralelizar validação** com Rayon
- [ ] **Buckets por conta** (hash % N) → paralelismo determinístico
- [ ] **Execução paralela dentro do bloco**
- [ ] **Batching no WAL** (fsync por tamanho/tempo)
- [ ] **Batch verify de assinaturas** (quando suportado)

---

## Consenso / Cluster

- [ ] **Leader-per-shard com lease** (menos reeleições)
- [ ] **Assinaturas agregadas (BLS ou similar)** para blocos/checkpoints
- [ ] **AppendEntries paralelos** por follower
- [ ] **Checkpoints periódicos** (root + sig agregada)

---

## Mempool / Network

- [ ] **Fila por conta** (um tx ativo por conta)
- [ ] **Anti-duplicata** (Bloom ou CRDT leve)
- [ ] **Hints de roteamento** → manda direto ao líder do shard
- [ ] **Prioridade** (tamanho/fee ou custo computacional)

---

## Camadas de Escala

- [ ] **Aggregator (rollup simples)**: empacotar 100–1000 tx, publicar root
- [ ] **Canais/sessões** (off-chain settlement)
- [ ] **Checkpoint opcional em L1 externo** (âncora)

---

## Observabilidade

- [ ] **Métricas TPS** (optimistic/final)
- [ ] **Latência p50/p95** (optimistic/final)
- [ ] **% revertidas no M0**
- [ ] **Spans críticos**: `validate_tx`, `exec_tx`, `wal_commit`, `append_entries`
- [ ] **Logs estruturados** (com node_id, term, index, req_id)

---

## Cliente / UX

- [ ] **Endpoints claros**
  - `POST /tx` → retorna `{status: accepted_optimistic}`
  - `GET /tx/{id}` → mostra estado atual
- [ ] **Tempo médio de reversão (M0)** exposto na API
- [ ] **Mensagens de status amigáveis** pro usuário final
