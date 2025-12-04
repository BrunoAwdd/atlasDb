# Project Status & Architecture: AtlasDB

*Última atualização: 23 de novembro de 2025*

Este documento resume a arquitetura e o estado atual do projeto `atlasDb`, com base em uma análise do código-fonte e das dependências.

## 1. Resumo

`atlasDb` é um **motor de replicação de estado genérico**, construído em Rust. Ele implementa um protocolo de consenso do tipo **Crash-Fault Tolerant (CFT)**, com uma arquitetura fortemente inspirada em protocolos como o **Raft**.

Seu objetivo é fornecer uma fundação para construir sistemas distribuídos consistentes, onde a lógica de negócio (o "conteúdo" das propostas) é desacoplada do mecanismo de consenso.

**Tecnologias Principais:**
- **Runtime Assíncrono:** `tokio`
- **Rede P2P:** `rust-libp2p`
- **API Externa:** `gRPC` (`tonic` e `prost`)

## 2. Arquitetura

O sistema é orquestrado por um componente central (`Maestro`) que gerencia um loop de eventos e coordena os seguintes módulos:

- **Orquestrador (`Maestro`):** O coração do nó. Ele gerencia o loop de eventos principal, conectando a rede, o consenso e a API. É responsável por iniciar/parar o servidor gRPC com base na liderança e por despachar eventos entre os módulos.
- **Consenso (`Cluster`):** Contém a lógica do algoritmo de consenso. Gerencia o estado do nó (líder/seguidor), realiza eleições, processa propostas, gerencia votos e mantém a consistência do log replicado.
- **Rede (`p2p`):** Camada de comunicação P2P baseada em `libp2p`. Cuida da descoberta de peers, disseminação de mensagens (propostas, votos) e comunicação direta entre os nós.
- **API (`gRPC`):** A interface para o cliente. Expõe o `ProposalService` e **só é executada no nó que detém a liderança do cluster**.
- **Armazenamento (`storage`):** Camada responsável pela persistência do log de propostas e do estado da máquina.

### Fluxo de uma Proposta

1.  O cliente envia uma proposta (`ProposalRequest`) para o endpoint gRPC do nó **líder**.
2.  O `Maestro` do líder recebe a requisição e cria uma estrutura `Proposal` completa (com ID, `parent`, etc.).
3.  O `Maestro` submete a proposta ao seu módulo `Cluster` local para validação.
4.  O `Cluster` retorna um comando para o `Maestro` disseminar a proposta na rede.
5.  O `Maestro` publica a proposta para todos os nós seguidores (followers) via `libp2p`.
6.  Os seguidores recebem a proposta, validam-na e enviam seus votos de volta ao líder.
7.  O líder coleta os votos. Ao receber quórum suficiente, considera a proposta "commitada" (finalizada) e a aplica à sua máquina de estado.

## 3. Checklist de Funcionalidades

Abaixo está um resumo dos conceitos de sistemas distribuídos que o `atlasDb` aborda e não aborda.

### ✅ Conceitos Abordados

- **Consenso (CFT, tipo Raft)**
- **Mecanismo de validação**
- **Ledger imutável (via log de propostas)**
- **Criptografia de chave pública**
- **Estrutura de eventos encadeados (`parent`)**
- **Finalidade (Finality)**
- **Execução determinística**
- **Máquina de estado (genérica)**
- **P2P networking (`libp2p`)**
- **Governança do protocolo (eleições, votação)**
- **Controle de tempo (Timers para eleição)**
- **Sincronização entre nós**
- **Verificabilidade independente**
- **Suporte a transações (genéricas)**
- **Mecanismo de armazenamento histórico**
- **Auditoria e rastreabilidade**
- **API de RPC (gRPC)**
- **Documentação do protocolo (`.proto`)**

### ❌ Conceitos Não Abordados

- **Tolerância a Falhas Bizantinas (BFT):** O sistema é CFT, não BFT.
- **Incentivos econômicos:** (Tokens, Staking, etc.)
- **Controle de gás/fees**
- **Suporte a smart contracts (sem VM)**
- **Compatibilidade com carteiras de usuário final**
- **Mecanismo de pruning ou compactação de histórico** (Não evidente)
