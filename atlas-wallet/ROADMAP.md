
## Labels e Issues (GitHub)
- Labels: `phase:0`…`phase:4`, `component:crypto`, `component:circuit`, `component:node`, `component:wallet`, `perf`, `security`, `spec`, `p2p`, `ux`
- Issues-tipo:
  - **feat:** implementação de API/feature
  - **bench:** medição e meta
  - **spec:** decisão/registro de protocolo
  - **sec:** análise de ameaça, review
  - **chore:** tooling, CI, fmt, lint

## Métricas-alvo (resumo)
- Prova 1→1: ≤ 1.5s | 1→2: ≤ 2.5s | 2→4: ≤ 4.0s
- TxClaim (1→1): ≤ 1.8KB | (2→4): ≤ 2.2KB
- Inserção Merkle: ≥ 50k/s em memória; ≥ 5k/s persistido
- Startup com snapshot (~1e6 leaves): < 2s

## Riscos e Mitigações
- **Tempo de prova elevado:** gadgets otimizados + limitar `k,m` + caching de witness
- **Fingerprint por valores:** bucketing + dummy outputs (v0.2)
- **Ligação temporal:** padding + delays + batching
- **Perda de senha do arquivo:** UX de backup de `vk` + alertas fortes

## Próximos passos imediatos (ToDo de arranque)
- [ ] Criar `crates/cvp-crypto` com scaffolding e tests (Rust 1.81+)
- [ ] Definir tags de domínio (cm, nf, memo) e constantes (H=32, V_MAX u64)
- [ ] Prototipar `cvp-file export/inspect` com Argon2id + ChaCha20-Poly1305
- [ ] Mock prover 1→1 (checks aritméticos) para integrar com a wallet
- [ ] Documentar formato do arquivo (MAGIC, headers, payload)
