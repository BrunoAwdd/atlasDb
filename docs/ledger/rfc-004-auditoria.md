# RFC-00Z — Auditoria Criptográfica e Reconstrução Determinística do Ledger Fiducial
Status: Draft  
Autor: Fernando Oliveira  
Versão: 0.1  
Data: 2025-12-11  

## 1. Objetivo
Definir o sistema de **auditoria criptográfica**, **provas de consistência**, **reconstrução determinística** e **validação econômica** do ledger Fiducial, permitindo que qualquer nó, auditor institucional ou provedor de custódia consiga verificar integralmente a contabilidade da rede.

O objetivo é estabelecer mecanismos formais que garantam:
- inexistência de criação arbitrária de valor  
- inexistência de destruição não autorizada  
- consistência contábil global  
- verificabilidade independente  
- reconstrução completa do estado a partir do log  

---

## 2. Escopo
Este RFC regula:
- Merkle proofs  
- Event Sourcing contábil  
- Provas de saldo (Proof of Balance)  
- Provas de passivo (Proof of Liabilities)  
- Provas de resultado  
- Reconstrução contábil determinística do estado  
- Auditoria modular para validadores e indexadores  

---

## 3. Princípios Fundamentais de Auditoria

### 3.1 Determinismo Total
Dado o mesmo conjunto de blocos e transações, **qualquer nó deve reconstruir exatamente o mesmo estado** econômico, patrimonial e de resultado.

### 3.2 Auditabilidade Local
Qualquer entidade deve ser capaz de provar seu próprio saldo, passivos e resultado sem depender de terceiros.

### 3.3 Auditabilidade Global
É possível verificar a consistência de todos os saldos, todas as contas e todos os períodos contábeis.

### 3.4 Proibição de Estados Órfãos
Nenhum estado pode existir sem trilha de auditoria (chain of custody contábil).

---

## 4. Estrutura Criptográfica

### 4.1 Merkle Tree do Estado Contábil
A Merkle Tree deve abranger não apenas os saldos, mas também:
- contas analíticas (ativo, passivo, receitas, despesas)  
- totalizadores por classe contábil  
- resultado acumulado  
- PL  

Cada nó da árvore representa:
```
hash( classe | conta | saldo | metadata )
```

### 4.2 Merkle Tree do Log Contábil (Event Log)
Todo evento contábil gera um registro:
```
{
  id,
  debit,
  credit,
  amount,
  nature,
  timestamp,
  metadata
}
```

A Merkle do log garante:
- imutabilidade  
- irreversibilidade  
- consistência histórica  

---

## 5. Módulos de Prova

### 5.1 Proof of Balance (PoB)
Permite provar que o saldo de uma conta é consistente com:
1. todos os lançamentos históricos  
2. resultado acumulado  
3. fechamento de período  

Requer:
- Merkle proof do estado  
- Merkle proof do log relevante  
- validação dupla: contábil + hash  

### 5.2 Proof of Liabilities (PoL)
Permite provar obrigações de contratos, pools, custódias e validadores.

Utiliza:
- contas de passivo  
- totalizadores por contrato  
- reconstrução determinística  

### 5.3 Proof of Result (PoR)
Usado para:
- receita acumulada  
- despesa acumulada  
- margem operacional do protocolo  

Processo:
1. validação dos lançamentos de receita/despesa  
2. conferência do fechamento  
3. verificação do PL resultante  

---

## 6. Reconstrução Determinística do Estado

### 6.1 Algoritmo Geral
Para reconstruir o ledger:

```
estado = estado_inicial
for evento in log_ordenado:
    aplicar(evento.debit += amount)
    aplicar(evento.credit -= amount)
    atualizar_contas_de_resultado(evento)
    validar_consistência(evento)
gerar_state_root()
```

### 6.2 Proibições
- Não pode existir conta sem origem contábil.  
- Não pode existir saldo sem trilha no log.  
- Não pode existir resultado sem contrapartida.  

---

## 7. Regras de Consistência

### 7.1 Igualdade entre Débitos e Créditos
Para cada bloco fechado:
```
Σ débitos == Σ créditos
```

### 7.2 Nenhuma Conta Pode Divergir do Log
Se:
```
hash(estado_reconstruído) != state_root
```
→ o bloco é inválido.

### 7.3 Resultado Compatível com PL
```
PL_n = PL_(n-1) + (Receitas - Despesas)
```

### 7.4 Auditoria Contínua
Cada validador deve:
- validar partidas dobradas  
- validar contrapartidas  
- validar classificação contábil  
- validar consistência do período  

---

## 8. APIs de Auditoria (Interface Externa)

### 8.1 `/audit/state`
Retorna:
- state_root  
- hash das classes contábeis  
- hash dos totalizadores  

### 8.2 `/audit/log/:txid`
Retorna Merkle proof do evento contábil.

### 8.3 `/audit/proof/balance/:address`
Retorna PoB (Proof of Balance).

### 8.4 `/audit/period/:epoch`
Retorna:
- resultado do período  
- receitas  
- despesas  
- PL  
- hash do fechamento  

---

## 9. Falhas de Auditoria (Detecção Automática)

O protocolo deve rejeitar blocos que apresentem:
- saltos contábeis  
- saldo incompatível com log  
- receitas sem origem  
- despesas sem contrapartida  
- PL negativo sem causa econômica  
- contas inexistentes no plano de contas  

---

## 10. Considerações Finais
Este RFC estabelece a fundação da **auditoria criptográfica contábil** no Fiducial, permitindo que a rede opere como um **sistema contábil distribuído, imutável, verificável e determinístico**, apto para instituições financeiras, custodiantes e auditores externos.

