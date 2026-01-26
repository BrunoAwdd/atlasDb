# Guia de Implantação Multi-No (Intranet)

Este guia explica como rodar o AtlasDB em vários computadores na mesma rede (Intranet/LAN) para que eles formem um cluster real, sem simulação local.

## Pré-requisitos

- 2 ou mais computadores conectados na mesma rede (Wi-Fi ou Cabo).
- **Windows:** É altamente recomendado usar o **WSL 2** (Ubuntu no Windows), pois o `rocksdb` é difícil de compilar nativamente no Windows.
- Rust instalado (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`).
- O código fonte do `atlas-wallet` copiado para todos.

## Configuração Windows (via WSL)

Se os outros PCs forem Windows, peça para instalarem o WSL:

1.  Abrir PowerShell como Admin e rodar: `wsl --install`
2.  Reiniciar o PC.
3.  Abrir o "Ubuntu" no menu Iniciar.
4.  Instalar dependências: `sudo apt update && sudo apt install build-essential libclang-dev protobuf-compiler`
5.  Instalar Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
6.  Clonar/Copiar o projeto para dentro do WSL (ex: `\\wsl.localhost\Ubuntu\home\usuario`).

## Passo 1: Descobrir os IPs

Em cada computador, descubra o seu endereço IP na rede local.

- **Linux/Mac:** `ifconfig` ou `ip a` (procure por 192.168.x.x ou 10.x.x.x)
- **Windows:** `ipconfig`

Vamos configurar o **Seu PC** como o **Bootstrap Node** (Ponto de Encontro).
Os outros computadores (Peers) vão se conectar a você para entrar na rede.

## Passo 2: Iniciar o Bootstrap Node (Seu PC)

No seu computador, rode o script passando apenas o seu IP:

```bash
# Exemplo: ./start_intranet_node.sh 192.168.1.5
./start_intranet_node.sh <SEU_IP_LOCAL>
```

**Muito Importante:**
O script vai iniciar e mostrar uma linha assim no topo:
`Local node identity is: 12D3KooW...`

Copie esse ID (`12D3KooW...`). Você vai precisar passar ele para todos os outros computadores.

## Passo 3: Iniciar os Outros Nós (Peers)

Nos outros computadores, você vai rodar o script apontando para o seu (Bootstrap):

```bash
# Sintaxe: ./start_intranet_node.sh <IP_DELE> <ID_DO_BOOTSTRAP> <IP_DO_BOOTSTRAP>

# Exemplo: O peer é 192.168.1.20 e vai conectar no seu (192.168.1.5) com ID 12D3Koo...
./start_intranet_node.sh 192.168.1.20 12D3KooW... 192.168.1.5
```

## Passo 4: Verificar Conexão

Se tudo der certo:

1.  Os logs de ambos devem mostrar `Peer descoberto: ...`.
2.  Eles devem começar a trocar "Heartbeats" (`❤️ HB de ...`).
3.  Se você enviar uma transação para o PC-1 (via Carteira Web apontando para 192.168.1.10:50051), o PC-2 também deve ver o bloco sendo produzido (Consenso).

## Solução de Problemas (Troubleshooting)

- **Firewall:** O problema mais comum é o firewall do sistema bloquear as portas 4001 (P2P) e 50051 (gRPC). Certifique-se de liberar essas portas ou desativar o firewall temporariamente para testar (`sudo ufw disable` no Ubuntu).
- **IP Incorreto:** Se o IP mudar (DHCP), você precisa ajustar o comando. Recomenda-se fixar o IP se possível.
- **Different Genesis:** Certifique-se de que ambos os PCs têm a mesma versão do código (especialmente `atlas-ledger`), pois as regras de genesis (saldos iniciais) são hardcoded no código por enquanto.
