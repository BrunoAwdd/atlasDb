# 🗺️ AtlasDB Chart of Accounts (Plano de Contas)

This document defines the strict numeric codes used by the Atlas Ledger.
All accounts must use the format: `CODE:NAME`.

## 1. ATIVO (Assets)

_Resources owned or controlled by the entity._

- **1.1** - Circulante (Current Assets)
  - 1.1.1 - Caixa e Equivalentes
  - 1.1.2 - Contas a Receber
- **1.2** - Não Circulante (Non-Current Assets)
  - 1.2.1 - Realizável a Longo Prazo
  - 1.2.2 - Investimentos
  - 1.2.3 - Imobilizado
  - 1.2.4 - Intangível

## 2. PASSIVO (Liabilities)

_Obligations to external parties._

- **2.1** - Circulante (Current Liabilities)
  - 2.1.1 - Fornecedores
  - 2.1.2 - Obrigações Fiscais
  - 2.1.3 - Obrigações com Clientes (Custódia)
- **2.2** - Não Circulante (Non-Current Liabilities)
  - 2.2.1 - Empréstimos LP

## 3. PATRIMÔNIO LÍQUIDO (Equity)

_Residual interest in the assets after deducting liabilities._

- **3.1** - Capital Social
- **3.2** - Reservas
- **3.3** - Ajustes de Avaliação Patrimonial

## 4. RESULTADO (Income Statement)

_Revenue and Expenses._

- **4.1** - Receitas Operacionais
- **4.2** - Custos (Operacionais e de Rede)
- **4.3** - Despesas Operacionais

## 5. CONTAS DE COMPENSAÇÃO (Off-Balance Sheet)

_Items that do not affect the balance sheet but require tracking._

- **5.1** - Ativos de Terceiros sob Custódia
- **5.2** - Garantias Prestadas e Recebidas

---

### 📝 Example Usage

| Account Name     | Ledger Key  | File Path                     |
| ---------------- | ----------- | ----------------------------- |
| **Alice Wallet** | `0x9239...` | `data/accounts/0x9239....bin` |
| **Bob Wallet**   | `0x2983...` | `data/accounts/0x2983....bin` |
