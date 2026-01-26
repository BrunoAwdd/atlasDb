# FIP-03 — Especificação de Bloco da Fiducial (Block & Header Specification)
*Fiducial Improvement Proposal 03*  
*Status: Draft*  
*Author: @fiducial-core*  
*Created: 2025-12-10*

---

## 1. Objetivo
Este documento define formalmente a **estrutura de bloco** da Fiducial Blockchain, incluindo:

- formato do cabeçalho (`BlockHeader`);
- regras para `block_hash` e `prev_hash`;
- assinatura do proponente;
- journal contábil por bloco;
- relação com `state_root` (FIP-01) e LedgerEntry (FIP-02);
- invariantes de validação.

O objetivo é padronizar como os blocos são construídos, validados e encadeados.

---

## 2. Princípios Fundamentais

### 2.1. Imutabilidade
Um bloco, após commitado, nunca pode ser modificado.

### 2.2. Encadeamento Criptográfico
Cada bloco referencia o anterior via `prev_hash`.  
Alterar qualquer bloco invalida todos os posteriores.

### 2.3. Prova de Autoria
Todo bloco é assinado pelo proponente (líder do consenso naquele round).

### 2.4. Determinismo
Qualquer nó honesto deve ser capaz de:
1. Validar um bloco apenas com dados locais;  
2. Reproduzir exatamente o mesmo `block_hash`.

---

## 3. Estrutura de Bloco

Um bloco é composto por:

```
Block {
  header: BlockHeader,
  journal: BlockJournal
}
```

---

## 4. BlockHeader

O cabeçalho contém todos os metadados essenciais para validação e encadeamento.

```
BlockHeader {
  height: u64,
  round: u64,
  proposer: Address,
  prev_hash: Hash,
  state_root: Hash,
  journal_root: Hash,
  timestamp: u64,
  signature: Signature,
  block_hash: Hash
}
```

### Descrição dos Campos

| Campo | Descrição |
|-------|-----------|
| **height** | Altura do bloco (genesis = 1). |
| **round** | Round do consenso BFT. |
| **proposer** | Endereço do nó proponente. |
| **prev_hash** | Hash do bloco anterior (encadeamento). |
| **state_root** | Compromisso do estado após este bloco (FIP-01). |
| **journal_root** | Raiz Merkle do conjunto de LedgerEntries. |
| **timestamp** | Epoch fornecido pelo proponente. |
| **signature** | Assinatura do cabeçalho sem `signature` e sem `block_hash`. |
| **block_hash** | Hash final do cabeçalho, calculado após assinatura. |

---

## 5. Cálculo do `journal_root`

O journal é o conjunto de entradas contábeis do bloco:

```
BlockJournal {
  entries: Vec<LedgerEntry>
}
```

A raiz do journal é definida como:

```
journal_root = MerkleRoot( serialize(entries) )
```

Cada `LedgerEntry` é serializado de forma determinística.

---

## 6. Cálculo do `block_hash`

O `block_hash` é definido como:

```
block_hash = H( serialize(header_without_signature_and_hash) || signature )
```

Onde:

- `header_without_signature_and_hash` exclui os campos:
  - `signature`
  - `block_hash`

Essa regra torna impossível alterar o bloco sem quebrar a cadeia.

---

## 7. Regras de Validação de Bloco

Um nó só aceita um bloco se TODAS as condições forem verdadeiras:

### 7.1. Regras Estruturais
- `height == previous_block.height + 1`
- `prev_hash == previous_block.block_hash`

### 7.2. Regras de Hash
- `block_hash` deve ser corretamente recalculável.
- `journal_root` deve corresponder ao Merkle root real do journal.
- `state_root` deve ser válido quando o executor aplicar o bloco.

### 7.3. Assinatura
- Assinatura deve ser válida para:
```
signature == Sign(proposer_private_key, header_without_signature_and_hash)
```

### 7.4. Consenso
O bloco deve ser:
- proposto pelo líder do round, ou
- aprovado via consenso BFT (pré-votos e commits).

### 7.5. Timestamp
`timestamp` deve obedecer às regras da rede (ex.: monotonicidade aproximada).

---

## 8. Processo de Construção de Bloco

### 1. Líder coleta transações válidas  
(ou propostas, dependendo do modo)

### 2. Executor aplica transações  
Gera:
- LedgerEntry
- Receipts
- Atualização do `State`

### 3. Gera:
- `journal_root`
- `state_root`

### 4. Constrói o cabeçalho sem assinatura  
### 5. Assina o cabeçalho  
### 6. Calcula `block_hash`  
### 7. Transmite para rede

---

## 9. Genesis Block (Bloco 1)

O bloco gênesis deve ter:

```
prev_hash = 0x0000...0000
state_root = MerkleRoot(State_0)
journal_root = MerkleRoot([])
```

Ele não segue as regras de consenso.

---

## 10. Invariantes do Protocolo

### 10.1. Integridade do Ledger
Alterar qualquer bloco invalida toda a cadeia.

### 10.2. Integridade Contábil
Um bloco só é válido se:
- Todas as LedgerEntries forem balanceadas (FIP-02).

### 10.3. Determinismo Total
Dado o mesmo state e o mesmo journal, qualquer nó deve produzir:
- o mesmo `state_root`,
- o mesmo `journal_root`,
- o mesmo `block_hash`.

---

## 11. Conclusão

O FIP-03 formaliza a estrutura do bloco da Fiducial, unificando:

- Ledger contábil (FIP-02),
- Estado global (FIP-01),
- Consenso BFT.

Esse documento cria a fundação para:

- validação determinística,
- auditabilidade total,
- reconstrução de estado,
- compatibilidade futura com light clients,
- evolução segura da rede.

---

## 12. Anexos Futuros Possíveis
- Layout binário da serialização (SSZ / RLP / Borsh / custom).  
- Provas Merkle para light clients.  
- Especificação de sincronização rápida (fast sync).  
