# RFC-OBS-001
## Observabilidade, Auditoria e Diagnóstico para Blockchain Autoral

### Status
Draft

### Data
2026-02-04

---

## 1. Objetivo

Este RFC define a arquitetura oficial de **observabilidade, auditoria e investigação**
para a blockchain autoral, contemplando:

- Execução **multi-nó em desenvolvimento** (simulação de consenso e nós maliciosos)
- Execução **mono-nó em produção**
- Comparação determinística de comportamento entre nós
- Auditoria forense sem impacto operacional contínuo
- Separação clara entre logs, métricas, binlogs e auditoria

O escopo deste documento é **operacional e técnico**.  
Regras de consenso e economia do protocolo estão fora deste RFC.

---

## 2. Princípios Arquiteturais

1. Métricas explicam o comportamento contínuo do sistema
2. Logs explicam exceções e desvios
3. Binlogs provam a história imutável
4. Auditoria é ativável, nunca permanente
5. Nenhuma análise crítica depende de leitura manual de arquivos

---

## 3. Tecnologias Adotadas

### 3.1 Observabilidade

- **Grafana OSS**
  - Visualização central
  - Dashboards comparativos entre nós
  - Alertas operacionais

- **Prometheus**
  - Coleta de métricas
  - Séries temporais
  - Base para alertas e tendência

- **Loki**
  - Armazenamento de logs
  - Indexação por labels
  - Investigação forense e comparativa

- **Promtail**
  - Coleta de logs baseados em arquivos
  - Suporte a JSON, regex e multiline

- **node_exporter**
  - Métricas do host (CPU, RAM, disco, I/O)

Todos os componentes são **open-source** e **self-hosted**.

---

## 4. Modalidades de Evidência

### 4.1 Log de Sistema (SYS / TX)

**Função**
- Diagnóstico operacional
- Registro de falhas, violações e comportamentos inesperados

**Formato**
- Texto livre

**Requisitos**
- Deve conter `height=` sempre que relacionado a consenso
- Deve conter `peer_id=` quando relacionado à rede

**Uso**
- Investigação pontual
- Correlação com auditoria e métricas

---

### 4.2 Auditoria de Votos

**Função**
- Análise de consenso
- Simulação de nós maliciosos
- Reconstrução lógica de decisões

**Formato**
- JSON estruturado

**Campos mínimos**
```json
{
  "height": 812,
  "peer_id": "peer_pubkey_hash",
  "vote_type": "prevote | precommit",
  "proposal_id": "block_hash",
  "result": "accepted | rejected",
  "reason": "string",
  "unix_ms": 1700000000000
}
```

**Política**
- Ativável por flag, env ou feature toggle
- Retenção curta
- Não constitui fonte de verdade

---

### 4.3 Binlog da Chain

**Função**
- Fonte de verdade sequencial
- Base para replay e verificação

**Formato**
- JSON (fase atual)
- Protobuf (fase futura)

**Características**
- Append-only
- Imutável
- Ordenado

**Observabilidade**
- Não ingerido integralmente no Loki
- Apenas eventos de marco são logados:
  - rotate
  - checksum failure
  - replay start/end
  - truncation

---

### 4.4 Binlog da Wallet

**Função**
- Registro contábil por conta
- Auditoria financeira
- Verificação de invariantes

**Formato**
- JSON → Protobuf

**Observabilidade**
- Logs apenas para inconsistências
- Métricas para taxa e latência de apply

---

### 4.5 Redb

**Função**
- Persistência de estado
- Possível gargalo de I/O

**Observabilidade**
- Métricas:
  - commit latency p95
  - read/write latency
  - tamanho em disco
- Logs apenas para erro e warning

---

## 5. Execução em Desenvolvimento (Multi-nó)

### 5.1 Topologia

- 4 nós executando localmente
- Stack única de observabilidade
- Logs segregados por nó

```
/var/log/blockchain/
  ├── node1/
  ├── node2/
  ├── node3/
  └── node4/
```

### 5.2 Identificação Obrigatória

Todo evento deve carregar:
- node_id
- height
- module
- kind

---

## 6. Execução em Produção (Mono-nó)

### 6.1 Política de Verbosidade

| Componente | Produção |
|----------|----------|
| SYS / TX | Mínimo |
| Auditoria de Votos | Desligada (ativável) |
| Binlogs | Sempre ativos |
| Métricas | Sempre ativas |

### 6.2 Regra Operacional

Se algo acontece sempre, vira métrica.  
Se não deveria acontecer, vira log.

---

## 7. Correlação e Investigação

### 7.1 Eixo Principal

- Height é o eixo universal de correlação

### 7.2 Comparação entre Nós

Investigação padrão:
- Mesmo height
- Eventos equivalentes
- Resultados divergentes

---

## 8. Política de Retenção (Dev)

| Modalidade | Retenção |
|----------|----------|
| SYS / TX | 7–14 dias |
| Auditoria de Votos | 1–7 dias |
| Binlogs | Conforme necessidade |
| Métricas | Meses |

---

## 9. Decisões Arquiteturais

- Logs não são fonte de verdade
- Binlogs não são observabilidade
- Auditoria não é contínua
- Métricas representam a saúde real do sistema

---

## 10. Próximos RFCs

- RFC-OBS-002: Feature Flags de Auditoria
- RFC-OBS-003: Métricas de Consenso
- RFC-OBS-004: Replay e Verificação Cruzada
- RFC-OBS-005: Migração JSON → Protobuf

---

## 11. Conclusão

A separação clara entre observar, auditar, provar e investigar
é condição necessária para manter a blockchain segura,
explicável e operacionalmente sustentável.
