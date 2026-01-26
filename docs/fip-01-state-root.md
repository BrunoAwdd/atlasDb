# FIP-01 — State Root: Compromisso Criptográfico do Estado Global
*Fiducial Improvement Proposal 01*  
*Status: Draft*  
*Author: @fiducial-core*  
*Created: 2025-12-10*

---

## **1. Objetivo**

Este documento define, de forma clara e formal, o significado do campo `state_root` na Fiducial Blockchain.  
O objetivo é estabelecer:

- o que é considerado **estado global** (“state”),
- como o `state_root` deve ser **conceitualmente calculado**,
- e qual o seu papel no **protocolo**, na **segurança** e na **recuperação do ledger**.

Este FIP **não exige implementação imediata** do cálculo real da raiz. Ele fixa o *contrato conceitual*, permitindo que a implementação técnica seja desenvolvida posteriormente.

---

## **2. Escopo**

Este FIP cobre:

- Estrutura lógica do **estado global** (`State`);
- Campo `state_root` do cabeçalho de bloco;
- Relação entre blocos, ledger/binlog e estado;
- Regras para reconstrução e verificação do estado.

Este FIP **não cobre**:

- detalhes específicos da árvore (Merkle, SMT, Jellyfish, etc.);
- mecanismos de execução de contratos inteligentes.

---

## **3. Definições**

### **3.1. State (estado global)**  
É o conjunto de todas as informações vivas necessárias para validar transações e continuar a evolução do ledger. Inclui, no mínimo:

- Saldos por conta e por ativo;
- `last_entry_id`: ponteiro para último lançamento contábil (dupla entrada);
- `nonce` por conta (em modelo account-based);
- Outros metadados essenciais para execução e validação futura.

### **3.2. Ledger / Binlog**  
A história imutável da blockchain: lista de blocos, cada um contendo:

- `height`, `hash`, `prev_hash`, `time`;
- payload (transações / proposals);
- `state_root` definido por este FIP.

### **3.3. `state_root`**  
É o **compromisso criptográfico** do estado global.  
Representa, em um único hash, a foto completa do estado após a aplicação do bloco.

---

## **4. Modelo Lógico do Estado (`State`)**

O estado é representado conceitualmente como:

```
State = {
  accounts: Map<Address, AccountState>,
}
```

Onde:

```
AccountState = {
  balances: Map<AssetId, u128>,
  last_entry_id: EntryId,
  nonce: u64,
}
```

Regras:

1. A coleção `accounts` deve ser considerada ordenada por `Address` para hashing.
2. Toda informação necessária à execução e validação deve estar no `State`.
3. Informações deriváveis exclusivamente do ledger **não fazem parte** do `State`.

---

## **5. Árvore de Estado (conceitual)**

O `state_root` é definido como a **raiz de uma árvore criptográfica** construída sobre o `State`.

### **5.1. Leaves**

Para cada `(address, account_state)` ordenado, define-se:

```
leaf_i = H( address || serialize(account_state) )
```

### **5.2. Merkle Root**

Os leaves são combinados recursivamente em pares até formar a raiz:

```
state_root = MerkleRoot(leaves)
```

Implementações futuras podem usar:

- Sparse Merkle Tree (SMT),
- Jellyfish Merkle Tree,
- Patricia Merkle Trie (PMT), ou equivalente.

---

## **6. Definição Formal do `state_root`**

Dado:

- Um bloco `B_h` na altura `h`;
- O estado `State_h` após aplicar todas as transações válidas do bloco sobre `State_{h-1}`;

Define-se:

```
state_root(h) = MerkleRoot( State_h )
```

E:

- O cabeçalho de `B_h` **deve** conter `state_root = state_root(h)`.

Para o bloco gênesis:

```
state_root(0) = MerkleRoot( State_0 )
```

---

## **7. Invariantes de Protocolo**

### **7.1. Imutabilidade**

Depois de commitado, o `state_root` não pode ser alterado.  
Mudar o estado de qualquer conta invalidaria:

- o hash do bloco,
- todos os blocos subsequentes.

### **7.2. Determinismo**

Reexecutar o binlog em qualquer nó deve resultar no mesmo `state_root` para cada altura.

### **7.3. Cobertura Completa**

Tudo que influencia transações futuras **deve estar incluído** no cálculo:

- saldos,
- nonces,
- ponteiros contábeis,
- aprovações futuras,
- storage de contratos (se houver).

---

## **8. Relação entre Ledger, State e `state_root`**

| Elemento | Natureza | Função |
|---------|----------|--------|
| **Ledger / Binlog** | História | Contém blocos e transações. |
| **State** | Estado vivo | “Foto atual” após aplicar o ledger. |
| **`state_root`** | Hash | Compromisso do estado após um bloco. |

Propriedade-chave:

> Se o estado for perdido, o nó pode reconstruí-lo **apenas reexecutando o ledger**, validando cada `state_root` ao longo do caminho.

---

## **9. Modos Temporários Permitidos (desenvolvimento)**

### **9.1. Modo DEV-ZERO**

```
state_root = 0x0000...0000
```

Usado para testes de gossip/consenso/binlog.

### **9.2. Modo DEV-MOCK**

```
state_root = H( height || block_hash || "dev" )
```

Permite variação visual sem computar Merkle real.

### **9.3. Modo PROD**

Cálculo real da árvore de estado.  
Este é o modo oficial para mainnet.

---

## **10. Reconstrução Completa do Estado**

Procedimento:

1. Inicia-se de `State_0`.
2. Para cada bloco `h`:
   - Aplicar as transações.
   - Obter `State_h`.
   - Calcular `state_root(h)`.
   - Comparar com o `state_root` do cabeçalho de `B_h`.
3. Divergência → erro crítico, possível corrupção ou fork inválido.

---

## **11. Extensões Futuras**

Este FIP permite incluir no estado:

- storage de contratos,
- intents,
- approvals,
- dados de custódia,
- parâmetros de execução.

Nada disso altera o significado do `state_root`.

---

## **12. Conclusão**

O `state_root` é a âncora criptográfica da Fiducial Blockchain.  
Ele garante:

- integridade,
- auditabilidade,
- reconstrução determinística,
- e consistência absoluta do estado global.

Este FIP fixa o significado conceitual do campo e padroniza seu uso em toda a arquitetura da Fiducial.

---

