# Ledger Completion Plan

## Goal

Completar a implementação dos módulos do `atlas-ledger` que estavam vazios (`bank` e `interface`).

## 1. Bank Module (`atlas-ledger/src/bank`)

O módulo Bank gerencia as regras da rede financeira, incluindo acesso institucional e verificações de conformidade.

### 1.1 Compliance Engine (`compliance_engine`)

**Responsabilidade**: Aplicar regras nas transações antes de serem executadas pelo Accounting Engine.

**Regras**:

- Receiver deve ter KYC válido (> Basic) para valores > limite.
- Sender não pode estar congelado.
- Limites de volume diário.

**Tarefas**:

- [ ] Criar trait `ComplianceRule`.
- [ ] Implementar `KycRule`: Checa níveis de KYC de sender/receiver.
- [ ] Implementar `ComplianceService`: Executa um conjunto de regras contra uma transação proposta.

### 1.2 Institution Core (`institution_core`)

**Responsabilidade**: Gerenciar entidades autorizadas (Bancos, Fintechs) que podem emitir ativos ou fazer onboarding de usuários.

**Tarefas**:

- [ ] Definir struct `Institution` (ID, Nome, Dados Públicos).
- [ ] Implementar `InstitutionRegistry`: Lista on-chain de bancos aprovados.

## 2. Interface Module (`atlas-ledger/src/interface`)

O módulo Interface fornece o gateway para clientes externos (Wallets, Exchanges) to interact with the Ledger.

### 2.1 gRPC Definitions (`proto`)

**Responsabilidade**: Definir o contrato de comunicação.

**Tarefas**:

- [ ] Criar `proto/ledger.proto`.
  - `SubmitTransaction(Transaction)`
  - `GetBalance(Account)`
  - `GetStatement(Account)`
- [ ] Configurar build script do `tonic` no `atlas-ledger`.

### 2.2 API Server (`api`)

**Responsabilidade**: Implementar a lógica do servidor gRPC.

**Tarefas**:

- [ ] Implementar `LedgerServiceImpl`.
- [ ] Conecta requisições gRPC ao `Ledger::execute_transaction` e `State`.
