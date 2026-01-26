# FIP-04 — Especificação do Mempool da Fiducial
*Fiducial Improvement Proposal 04*  
*Status: Draft*  
*Author: @fiducial-core*  
*Created: 2025-12-10*

---

## 1. Objetivo
Este documento padroniza o funcionamento do **Mempool da Fiducial**, definindo:

- critérios de aceitação de transações,
- rejeições imediatas,
- política de prioridade,
- interação com o consenso,
- interação com o RPC,
- regras de expiração,
- anti-spam e DoS minimization,
- suporte a transações patrocinadas (Sponsored Tx).

O mempool é a **fila local e não-consensual** de transações pendentes.

---

## 2. Princípios Fundamentais

### 2.1. O Mempool NÃO é parte do consenso
Cada nó mantém seu próprio mempool.  
Consenso valida blocos, não mempools.

### 2.2. O Mempool NÃO é parte do ledger
Ele contém apenas intenções, não fatos contábeis.

### 2.3. Stateless-first, Stateful-light
A maior parte das rejeições deve ocorrer **antes** de tocar o estado:
- assinatura inválida,
- nonce negativo,
- campos faltantes,
- formatação incorreta.

Regras *stateful* são aplicadas de forma leve:
- saldo mínimo para taxa,
- nonce plausível.

### 2.4. O Propositor decide
O líder do round escolhe livremente quais transações incluir.

---

## 3. Estrutura do Mempool

```
Mempool {
  entries: Map<TxHash, MempoolEntry>,
  by_sender: Map<Address, Vec<TxHash>>,
  priority_queue: MaxHeap<(FeePriority, TxHash)>
}
```

### 3.1. MempoolEntry

```
MempoolEntry {
  tx: Transaction,
  arrival_timestamp: u64,
  gas_price: u128,
  fee_priority: u128,
  sender: Address,
  nonce: u64
}
```

---

## 4. Critérios de Aceitação

### 4.1. Validação Stateless (obrigatória)
- assinatura válida,
- chain_id correto,
- nonce >= nonce_on_chain,
- tamanho máximo respeitado.

### 4.2. Checagem Stateful
- saldo suficiente para gás,
- nonce dentro de janela aceitável.

### 4.3. Anti-Spam
- limite por conta,
- limite global do mempool.

---

## 5. Política de Prioridade

```
priority = gas_price
```

Ou:

```
priority = gas_price * gas_limit
```

FIFO entre iguais.

---

## 6. Interação com o Consenso

- líder coleta top-N transações,
- validadores não precisam ter as mesmas transações,
- txs incluídas são removidas,
- txs de nonce ultrapassado são removidas.

---

## 7. RPC

- sendTransaction  
- getTransactionStatus  
- getPendingTransactions  
- WebSocket events: txPending, txDropped, txInBlock, txConfirmed  

---

## 8. Expiração

- timeout por tempo (ex.: 300s),
- nonce ultrapassado,
- falta de saldo,
- replace-by-fee.

---

## 9. Sponsored Transactions

Campos adicionais:
- sponsor  
- max_sponsor_fee  

Regras:
- sponsor paga gás,
- sender assina operação,
- ambos podem assinar.

---

## 10. Gossip

- anúncios de novas txs,
- filtros locais,
- não precisa uniformidade.

---

## 11. Segurança

- limites por IP,
- limites por sender,
- min fee dinâmica,
- drop automático sob pressão.

---

## 12. Conclusão

O FIP-04 formaliza o mempool como parte essencial da pipeline:

Wallet → RPC → Mempool → Executor → Ledger → Block → State

---

## 13. Próximos FIPs
- FIP-05 — Especificação da Transação  
- FIP-06 — Execução Determinística  
- FIP-07 — Sponsored Payments (RIP-01)  
