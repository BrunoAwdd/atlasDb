# RFC-00X — Princípios Contábeis do Ledger Fiducial
Status: Draft  
Autor: Fernando Oliveira  
Versão: 0.1  
Data: 2025-12-11  

## 1. Objetivo
Definir os princípios contábeis obrigatórios que regem todas as operações registradas na blockchain **Fiducial / AtlasDB**, estabelecendo um modelo de escrituração distribuída baseado em **partida dobrada**, **contrapartida**, **competência**, **periodicidade**, **segregação patrimonial**, **auditoria criptográfica** e **plano de contas padronizado**.

## 2. Escopo
Aplicável a transações financeiras, operações internas do protocolo, módulos de custódia, mecanismos de consenso e indexadores.

## 3. Princípios Contábeis do Ledger

### 3.1 Partida Dobrada
Toda transação gera lançamento duplo: débito e crédito obrigatórios.

### 3.2 Conta Contrapartida
Nenhuma operação pode existir sem contrapartida. Naturezas mínimas: fees, burns, staking, slashing.

### 3.3 Imutabilidade e Lançamento Inverso
Nenhum lançamento é removido; correções ocorrem via operação inversa.

### 3.4 Contas de Resultado
Contas internas: receitas (fees, spreads), despesas (burn, staking, ajustes).

### 3.5 Periodicidade
Fechamento automático por epoch, transferindo resultado para PL.

### 3.6 Competência
Eventos econômicos são registrados no momento em que ocorrem.

### 3.7 Segregação Patrimonial
Cada wallet/contrato é entidade contábil independente.

### 3.8 Auditoria Criptográfica
Uso de Merkle proofs, event sourcing, proof of balance, reconstrução determinística.

### 3.9 Plano de Contas
Estrutura mínima: ativo, passivo, PL, receitas, despesas.

## 4. Modelo de Lançamento
Cada transação gera estrutura:
```
{
  id,
  timestamp,
  debit_account,
  credit_account,
  amount,
  nature,
  metadata
}
```

## 5. Regras de Consistência
Débitos = créditos; nenhuma conta pode ir negativa salvo passivo; toda operação deve seguir plano de contas.

## 6. Integração com Consensus
Validadores rejeitam transações sem contrapartida, assimétricas ou inconsistentes.

## 7. Considerações Futuras
IFRS-Chain, multicurrency, auditoria externa on-chain.

## 8. Conclusão
Este RFC formaliza a transformação do Fiducial em um **ledger contábil descentralizado**, garantindo integridade e auditabilidade global.
