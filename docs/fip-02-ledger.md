# FIP-02 — Ledger Contábil da Fiducial
*Fiducial Improvement Proposal 02*  
*Status: Draft*  
*Author: @fiducial-core*  
*Created: 2025-12-10*

---

## 1. Objetivo
Padronizar o funcionamento do **Ledger Contábil da Fiducial**, definindo princípios, estrutura de lançamentos, regras de dupla entrada e sua relação com o estado e o binlog.

## 2. Princípios Fundamentais
- **Imutabilidade**: nenhum lançamento é apagado.  
- **Dupla entrada**: todo evento deve balancear débitos e créditos.  
- **Não-reversão**: estornos são feitos via lançamentos compensatórios.  
- **Journal por bloco**: cada bloco é um livro diário.  
- **Determinismo**: o ledger deve ser suficiente para reconstruir todo o estado.

## 3. Estrutura do Ledger

### 3.1 LedgerEntry
```
LedgerEntry {
  entry_id: EntryId,
  legs: [
    { account: Address, asset: AssetId, kind: Debit,  amount: u128 },
    { account: Address, asset: AssetId, kind: Credit, amount: u128 }
  ],
  tx_hash: Hash,
  memo: String?,
  block_height: u64,
  timestamp: u64,
  prev_for_account: Map<Address, EntryId>
}
```

### 3.2 Leg
```
Leg {
  account: Address,
  asset: AssetId,
  kind: Debit | Credit,
  amount: u128
}
```

Regras:
- Débito reduz saldo; crédito aumenta.  
- Total de débitos = total de créditos.

## 4. Journal de Bloco
```
BlockJournal {
  height: u64,
  entries: Vec<LedgerEntry>
}
```

## 5. Fluxo de Execução Contábil
1. Executor valida a transação  
2. Cria LedgerEntry  
3. Atualiza AccountState  
4. Gera receipt  
5. Insere no bloco  
6. Calcula novo state_root  

## 6. Estorno
Lançamento compensatório:
```
Debit(B,100)
Credit(A,100)
```

## 7. Relação Ledger → State
```
AccountState {
  balances: Map<AssetId, u128>,
  last_entry_id: EntryId,
  nonce: u64
}
```

Atualização:
- Débito → saldo - amount  
- Crédito → saldo + amount  
- last_entry_id atualizado para entry_id

## 8. Receipts
```
TransactionReceipt {
  tx_hash,
  status,
  ledger_entry_ids,
  gas_used,
  memo
}
```

## 9. Conclusão
O ledger da Fiducial é oficialmente definido como um **livro-razão de dupla entrada**, transformando a blockchain em um sistema financeiro auditável, determinístico e compatível com práticas contábeis globais.

