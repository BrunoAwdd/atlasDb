# RFC 001: Account Event Chain (AEC) — Protocol Specification

**Status:** Draft 1.0 (Engineering Ready)\*\*  
**Context:** Core Protocol / Storage Engine\*\*  
**Architect:** [Nome do Protocolo]\*\*  
**Date:** 2024-05-21\*\*

## 1. Visão Geral

Esta RFC define a arquitetura **Account Event Chain (AEC)**.  
Diferente de blockchains monolíticas como Ethereum — onde o histórico é um log global misturado — este protocolo implementa **Event Sourcing nativo**, no qual **cada conta possui sua própria linha do tempo linear, encadeada e verificável**.

## 1.1 O Axioma

> **“O estado on-chain é apenas um cursor de validação.  
> A verdade completa reside nos segmentos de eventos históricos distribuídos.”**

## 1.2 Objetivos Técnicos

- Eliminar Indexadores: O protocolo deve fornecer histórico pesquisável nativamente.
- Leitura O(M): A complexidade de leitura depende apenas do histórico da conta, não do tamanho da chain global.
- Escrita O(1): A complexidade de escrita por Tick é constante, independente do volume de transações (via Batching).

## 2. Camada de Estado (On-Chain State)

O AccountState é mantido na RAM dos validadores (Hot State). Ele deve ser extremamente leve, funcionando apenas como um ponteiro de integridade.

```rust
struct AccountState {
    last_event: {
        tick: u64,
        hash: Hash256
    },
    balance_hint: u128
}
```

## 3. Camada de Armazenamento (Physical Storage)

Os dados históricos são persistidos em arquivos imutáveis chamados Segmentos (.bin).

### 3.1 Estrutura do Segmento (.bin)

Para evitar fragmentação do sistema de arquivos (milhões de arquivos pequenos), os eventos são agrupados em segmentos sequenciais.
Especificação do Arquivo:

- Nome: segment*{start_tick}*{end_tick}.bin
- Encoding: Little Endian
- Compressão: Zstd (Block Level)

| Offset | Tamanho  | Campo       | Descrição    |
| ------ | -------- | ----------- | ------------ |
| 0x00   | 4 bytes  | MAGIC       | Assinatura   |
| 0x04   | 2 bytes  | VERSION     | Versão       |
| 0x06   | 8 bytes  | START_TICK  | Tick inicial |
| 0x0E   | 8 bytes  | END_TICK    | Tick final   |
| ...    | Var      | EVENT_BATCH | Evento       |
| EOF-32 | 32 bytes | CHECKSUM    | Blake3       |

### 3.2 Indexação Local

Cada Nó mantém um índice KV (Key-Value) leve (ex: RocksDB ou redb) para mapear hashes lógicos para locais físicos. Este índice não participa do consenso, é apenas para aceleração de leitura.

```
Key:   (AccountHash + Tick)
Value: (SegmentID, Offset, Length)
```

## 4. Batch-per-Tick

Para resolver o problema de "Hot Accounts" (gargalo sequencial em contas de alto volume), o protocolo adota a regra de Um Evento por Tick.

- Aggregation: Durante o Tick $T$, todas as transações destinadas à conta 0xA são acumuladas em memória.
- Reduction: O protocolo consolida as operações (somas de saldos, chamadas de contrato).
- Commit: Ao fechar o Tick $T$:
  - Gera-se um único AccountEvent contendo o vetor de operações.
  - Calcula-se o TensorHash.
  - Atualiza-se o AccountState atomicamente.

## 5. Modelo de Integridade

A integridade da cadeia é garantida por um Tensor Accumulator (ou Hash Chain aprimorada), que vincula o conteúdo ao tempo e ao evento anterior.

```
H_current = TensorHash(PrevHash, Tick, PayloadVector)
```

- PrevHash: Garante a ordem imutável (Linked List).
- Tick: Garante a timestamp determinístico.
- Payload: Garante o conteúdo das transações.

Isso permite que um cliente valide qualquer evento isolado apenas possuindo o hash do evento seguinte.

## 6. Protocolo de Recuperação (Read Path)

O cliente (Wallet/App) opera em modo "Lazy Loading", baixando apenas o necessário.

### 6.1 Reverse Chaining

1. Handshake: Cliente pede GetState(0xAlice). Node retorna last_event.
2. Fetch: Cliente pede o corpo do evento via Hash ou Tick.
3. Link: Cliente lê prev_hash dentro do evento e repete o processo para trás até preencher a UI (ex: últimos 20 itens).

### 6.2 Streaming

Para auditorias ou "Full Sync":

- Request: `GET /stream?account=0xAlice&from=0&to=1000`
- Process: O Node localiza os segmentos no disco e utiliza Zero-Copy sendfile para enviar o fluxo de bytes brutos diretamente para a rede.

## 7. Análise Competitiva

| Característica | EVM         | AEC            |
| -------------- | ----------- | -------------- |
| Estado         | Trie Pesada | Cursor Leve    |
| Histórico      | Logs        | Lista Ligada   |
| Concorrência   | Sequencial  | Batch-per-Tick |
| Auditoria      | Difícil     | Instantânea    |

## 8. Conclusão

A arquitetura AEC move a complexidade da computação global para o armazenamento local estruturado. Ao tratar o histórico como uma estrutura de dados de primeira classe (via Segmentos .bin e Batch-per-Tick), o protocolo viabiliza aplicações financeiras auditáveis e de alta performance sem infraestrutura intermediária.

## 9. Questões em Aberto (Open Questions)

Esta seção documenta decisões de engenharia ainda não finalizadas.
Elas não impedem a implementação do AEC, mas exigem análise futura, benchmarks ou novas RFCs complementares.

### 9.1 Segment Lifecycle (Fechamento de Segmentos)

- O segmento .bin deve ser fechado após quantos MB?
- Deve haver um limite de número de eventos por segmento?
- Segmentos seguem limites de epoch/slot do consenso?
- Devem existir segmentos “quentes” e “frios” para otimização de disco?

**Status:** Aberto
**Requer:** Benchmark de disco / IOPS / latência de leitura

### 9.2 Inclusão de last_event.hash na Árvore Global de Estado

- O hash é parte obrigatória da Merkle/SMT root?
- Ou é apenas um metadado local por validador?
- Em qual RFC será definida a árvore global (SMT, Jellyfish, Verkle, etc.)?

**Status:** Aberto
**Requer:** RFC-002 (State Tree Design)

### 9.3 Semântica de Contratos Inteligentes

- Como os contratos em execução no mesmo Tick empacotam suas operações no Batch?
- Eventos internos entram como subpayload?
- Um contrato pode gerar múltiplos subeventos dentro do mesmo Batch?

**Status:** Aberto
**Requer:** RFC-003 (Execution Environment / VM semantics)

### 9.4 Ticks sem eventos

- Se uma conta não recebe transações por milhões de Ticks, seu estado permanece estável?
- É necessário “avançar” last_event ou isso é completamente irrelevante?
- Há implicações para pruning ou aceleração de leitura?

**Status:** Aberto
**Requer:** Discussão com time de client-side indexing

### 9.5 Especificação do TensorHash

- Qual a função final: Blake3 + matriz? Poseidon? FNV-1a?
- O TensorHash pode operar como PoH compressivo?
- Deve existir salting por Tick ou apenas concatenação?

**Status:** Aberto
**Requer:** RFC-004 (Tensor Accumulator)

## 9.6 Consenso e Validação

- Em qual etapa o validador computa o TensorHash?
- O hash deve ser validado pelos peers na gossip layer?
- Existe um “Event Proof” para light clients?

**Status:** Aberto
**Requer:** RFC-005 (Consensus Integration)

### 9.7 Formato de Serialização do Payload

- CBOR? Borsh? Protobuf? Custom binary format?
- O payload pode ser certificadamente compatível para futuras versões?

**Status:** Aberto
**Requer:** RFC-006 (Canonical Serialization)

### 9.8 Distribuição de Segmentos (.bin) via CDN/IPFS

- Segmentos devem ser publicados automaticamente?
- O cliente pode escolher fontes múltiplas (peering)?
- É necessário um "Segment Discovery Protocol"?

**Status:** Aberto
**Requer:** RFC-007 (Storage & Distribution Layer)
