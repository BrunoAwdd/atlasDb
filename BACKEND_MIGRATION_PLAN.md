# Implementation Plan: Backend-Driven Accounting Classification

**Goal**: Move all accounting logic and classification (Active, Passive, Equity + Subgroups) from Frontend to Backend. The Frontend should be a "Dump Viewer" (Render only), while the Ledger/Node dictates the accounting structure.

## 1. Context & Problem

Currently, the Frontend "guesses" the classification based on prefixes (`vault:`, `wallet:`) and symbols (`ATLAS`, `USD`).

- **Issues**:
  - Fragile and duplicated logic across Wallet/Explorer.
  - "Guesswork" (Heuristics) instead of deterministic accounting.
  - Subgroups (e.g., "3.1 Capital Social" vs "3.2 Reserves") are hardcoded in UI.

## 2. Proposed Architecture

### A. Ledger Layer Updates

The Ledger should explicitly know the "Nature" of an account or asset.

1. **Account Metadata**:
   - Classify accounts upon creation (Genesis or `create_vault`).
   - Store `AccountNature`: `UserWallet`, `SystemVault`, `IssuanceVault`.

2. **Asset Metadata**:
   - Store `AssetClass`: `EquityToken` (ATLAS), `Currency` (USD), `Commodity`.

### B. API Layer (`atlas-node`)

The `/api/balance` endpoint should return a fully classified Financial Statement view.

**New Response Structure:**

```json
{
  "address": "...",
  "statement": {
    "section": "equity",         // "active", "passive", "equity"
    "group": "3.1 Capital Social", // "1.1 Cash", "2.1 Deposits"
    "is_credit_nature": true     // Helpful for UI coloring
  },
  "view": { ... } // Detailed split
}
```

### C. Implementation Steps

#### Phase 1: Hardcoded Backend Map (Short Term)

Instead of dynamic inference, implementation a definitive `AccountingMap` in Rust.

- **Input**: `Address` + `AssetID`.
- **Output**: `Section` (Active/Passive/Equity) + `Subgroup` (Label).
- This replaces the logic I just added to `rest.rs` with a more granular one.

#### Phase 2: Binlog/Ledger Integration (Long Term)

- Utilize the `kind` (Debit/Credit) from the Transaction Log (`binlog`) to reconstruct the nature of the balance.
- If an account primarily receives `Credits` of `EquityTokens`, it is an Equity Account.
- If an account receives `Debits` of `Currency`, it is an Asset Account.

## 3. Immediate Action (Post-Presentation)

1. Keep the current "Stable" version (Frontend + simple Backend classification) for the demo.
2. **Next Sprint**: Refactor `atlas-node` to return the `groups` object explicitly.

## 4. Verification

- **Test**: The "Raw JSON" must contain the correct "3.1 Capital Social" label for ATLAS in `vault:issuance`.
- **Test**: Frontend should have ZERO logic about "Reserves" or "Capital". It just renders `json.group`.
