# RFC-00W — Accounting Engine do Ledger Fiducial  
Status: Draft  
Autor: Fernando Oliveira  
Versão: 0.1  
Data: 2025-12-11  

## 1. Objetivo
Definir o funcionamento interno do **Accounting Engine**, responsável por transformar cada transação do protocolo em **lançamentos contábeis duplos**, validar natureza econômica, aplicar competências, atualizar contas analíticas e produzir reflexos patrimoniais e de resultado.  
Este módulo é o núcleo da contabilidade distribuída do Fiducial.

---

## 2. Escopo
O Accounting Engine cobre:

- classificação de natureza econômica  
- geração de débito/crédito  
- validação do plano de contas  
- atualização de resultado e PL  
- integração com consenso  
- tratamento de exceções e reversões  
Não cobre storage, rede, P2P ou Merkle Trees (estes são tratados em outros RFCs).

---

## 3. Arquitetura Geral
Toda transação processada passa por quatro fases:

1. **Interpretação Econômica**  
   Analisa natureza, tipo e implicações contábeis.

2. **Escrituração**  
   Gera um lançamento duplo:  
   ```
   D <conta_debito>
   C <conta_credito>
   ```

3. **Postagem (Posting)**  
   Atualiza saldos das contas e totalizadores.

4. **Validação de Consistência**  
   Verifica:  
   - integridade da partida dobrada  
   - conformidade com plano de contas  
   - compatibilidade com estado anterior  

---

## 4. Estrutura da Transação Econômica
Cada transação deve ser convertida em um objeto interno:

```
{
  id,
  amount,
  from,
  to,
  nature,        // ex: transfer, fee, staking_reward
  timestamp,
  metadata
}
```

O motor contábil **não interpreta semântica financeira**, apenas natureza econômica.

---

## 5. Tabela de Natureza Econômica
A natureza define qual conta será debitada e qual será creditada. Exemplos:

### 5.1 Transferência
```
D ativo:wallet:destinatario
C ativo:wallet:remetente
```

### 5.2 Fee
```
D ativo:wallet:usuario
C receita:fees
```

### 5.3 Staking Reward
```
D despesa:staking_rewards
C ativo:wallet:usuario
```

### 5.4 Burn
```
D despesa:burn
C ativo:supply:burned
```

### 5.5 Slashing
```
D ativo:validator:<id>
C receita:protocol:operacoes
```

Todas as naturezas devem ser declaradas no plano de contas.

---

## 6. Ciclo de Execução da Transação

### 6.1 Passo 1 — Classificação
O engine determina:
```
conta_debito  = mapping[nature].debito
conta_credito = mapping[nature].credito
```

### 6.2 Passo 2 — Lançamento
Criação do registro contábil:
```
lançamento = {
  debit: conta_debito,
  credit: conta_credito,
  value: amount,
  timestamp
}
```

### 6.3 Passo 3 — Validação
- ambas as contas devem existir  
- a natureza deve ser reconhecida  
- não pode resultar em saldo negativo não permitido  
- débito != crédito proibido  
- contas de classes erradas são rejeitadas  

### 6.4 Passo 4 — Aplicação
Aplicar efeitos:
```
saldo[conta_debito]  += amount
saldo[conta_credito] -= amount
```

### 6.5 Passo 5 — Atualização de Resultado
Se a conta for de receita ou despesa:
- acumular em totalizadores
- atualizar resultado do período

---

## 7. Mecanismo de Reversão (Rollback Contábil)
Nunca apaga registros. Sempre gera um lançamento inverso:

### 7.1 Exemplo:
Original:
```
D ativo:wallet:a
C ativo:wallet:b
```

Reversão:
```
D ativo:wallet:b
C ativo:wallet:a
```

Validador deve aceitar apenas se originado por evento autorizado.

---

## 8. Integração com o Consensus
O consenso só considera um bloco válido se:

1. Todas as transações são contábilmente válidas  
2. Σ débitos do bloco == Σ créditos do bloco  
3. Nenhum saldo final viola regras do plano de contas  
4. O resultado acumulado fecha corretamente  
5. O hash do estado contábil final condiz com o state_root calculado  

O Accounting Engine fornece ao consenso:
- totalizadores
- delta de resultado
- novo PL
- state_root contábil parcial

---

## 9. Tabela de Erros Econômicos

| Código | Descrição |
|-------|-----------|
| ECON-001 | Conta não existe no plano de contas |
| ECON-002 | Natureza não reconhecida |
| ECON-003 | Conta não pode receber débito |
| ECON-004 | Conta não pode receber crédito |
| ECON-005 | Saldo insuficiente |
| ECON-006 | Débito ≠ Crédito |
| ECON-007 | Classificação incompatível |
| ECON-008 | Resultado negativo inconsistente |
| ECON-009 | Contrapartida ausente |
| ECON-010 | Conta de usuário usada como receita/despesa |

---

## 10. Fechamento de Período
O engine deve expor endpoint interno:

```
close_period(epoch):
    resultado = receitas - despesas
    D receita:*   C PL
    D PL          C despesa:*
```

Gerando:
- snapshot do estado  
- hash de fechamento  

---

## 11. Hooks Públicos do Engine
### 11.1 `classify(tx)`
Retorna o débito/crédito propostos.

### 11.2 `post(entry)`
Aplica lançamento ao estado.

### 11.3 `rollback(entry)`
Aplica reversão contábil.

### 11.4 `validate(state, entry)`
Valida regras de consistência.

---

## 12. Considerações Finais
O Accounting Engine oficializa o Fiducial como **primeira blockchain com contabilidade nativa**, garantindo:

- integridade econômica  
- auditabilidade total  
- impossibilidade de fraude contábil  
- separação patrimonial absoluta  
- reconstrução determinística  

Este módulo é o coração do ecossistema contábil distribuído.

