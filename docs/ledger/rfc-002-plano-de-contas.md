# RFC-00Y — Plano de Contas Padronizado do Ledger Fiducial
Status: Draft  
Autor: Fernando Oliveira  
Versão: 0.1  
Data: 2025-12-11  

## 1. Objetivo
Estabelecer um **Plano de Contas Universal** para o ledger Fiducial, garantindo padronização econômica, auditoria determinística e classificação consistente em todas as operações on-chain.

## 2. Estrutura Geral
O plano de contas segue cinco classes principais:

1. **Ativo (A)**
2. **Passivo (P)**
3. **Patrimônio Líquido (PL)**
4. **Receitas (R)**
5. **Despesas (D)**

Cada classe abriga contas sintéticas e analíticas.

---

## 3. Classe 1 — Ativo (A)
Representa recursos econômicos controlados por entidades on-chain.

### 3.1 Ativo Circulante
- `ativo:wallet:<address>`
- `ativo:custodia:<vault>`
- `ativo:contratos:<contract_id>`
- `ativo:reservas:liquidez`

### 3.2 Ativo Não Circulante
- `ativo:lastro:<asset_id>`
- `ativo:penhoras`
- `ativo:provisões:creditos_receber`

---

## 4. Classe 2 — Passivo (P)
Representa obrigações exigíveis do protocolo ou de contratos.

### 4.1 Passivo Circulante
- `passivo:obrigações:<contract>`
- `passivo:liquidacao_pendente`
- `passivo:staking:devolver`
- `passivo:swap:reserva`

### 4.2 Passivo Não Circulante
- `passivo:contratos:lockup`
- `passivo:emissoes_futuras`

---

## 5. Classe 3 — Patrimônio Líquido (PL)
Base econômica da entidade “protocolo”.

- `pl:capital_inicial`
- `pl:resultados_acumulados`
- `pl:ajustes_contabeis`

Resultados são incorporados ao PL ao final de cada período contábil.

---

## 6. Classe 4 — Receitas (R)
Entradas econômicas geradas pelo funcionamento da chain.

- `receita:fees`
- `receita:mint_spread`
- `receita:protocol:operacoes`
- `receita:juros:provisionados`
- `receita:rollback_recuperado`

---

## 7. Classe 5 — Despesas (D)
Saídas econômicas ou destruição intencional de valor.

- `despesa:burn`
- `despesa:staking_rewards`
- `despesa:slashing`
- `despesa:ajustes`
- `despesa:rollback`

---

## 8. Regras de Classificação
1. Toda transação deve mapear **exatamente** a uma conta de débito e uma de crédito.  
2. Contas de usuário são sempre **Ativo**.  
3. Contratos podem ser **Ativo** ou **Passivo**, conforme natureza.  
4. Eventos do protocolo devem classificar-se em **Receitas** ou **Despesas**.  
5. Nenhuma transação pode criar conta nova fora do plano definido.  

---

## 9. Mapeamento por Natureza de Operação
### 9.1 Transferência
```
D ativo:wallet:destinatário
C ativo:wallet:remetente
```

### 9.2 Fee
```
D ativo:wallet:usuario
C receita:fees
```

### 9.3 Burn
```
D despesa:burn
C ativo:supply:queimado
```

### 9.4 Staking Reward
```
D despesa:staking_rewards
C ativo:wallet:usuario
```

### 9.5 Slashing
```
D ativo:validator:<id>
C receita:protocol:operacoes
```

---

## 10. Governança do Plano de Contas
- Alterações exigem **proposta de governança**, revisão econômica e validação técnica.  
- Contas novas devem manter sintaxe padronizada:  
`<classe>:<categoria>:<identificador>`  

---

## 11. Conclusão
O Plano de Contas Universal estabelece a fundação necessária para um **ledger contábil descentralizado**, permitindo auditoria contínua, classificação automática de eventos e consistência global entre validadores e indexadores.

