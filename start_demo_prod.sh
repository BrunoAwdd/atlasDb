#!/bin/bash

# Script de Suporte - MODO PROD (AtlasDB)
# Executa o Cluster com Cloudflare Tunnel URLs

echo "============================================="
echo "🚀 INICIANDO MODO PROD - ATLAS DB"
echo "============================================="

# Load NVM for npm access
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

# Iniciar Cloudflare Tunnel
echo "🌐 Conectando Cloudflare Tunnel..."
if pgrep -x "cloudflared" > /dev/null; then
    echo "⚠️  Tunnel já rodando."
else
    cloudflared tunnel run at-tunnel > tunnel.log 2>&1 &
    TUNNEL_PID=$!
    echo "✅ Tunnel iniciado (PID: $TUNNEL_PID)"
fi

sleep 3

# Navegar para o projeto
cd /home/bruno/projects/atlasDb

# Limpar dados antigos
echo "🧹 Limpando dados antigos..."
chmod +x clean_data.sh
./clean_data.sh

# Build do Node
echo "🔨 Building atlas-node..."
export RUST_LOG=info
cargo build -p atlas-node --bin atlas-node --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

BIN="./target/release/atlas-node"

# Cleanup Function
cleanup() {
    echo "🛑 Parando todos os processos..."
    kill $PID_1 $PID_2 $PID_3 $PID_4 $PID_WALLET $PID_EXPLORER $TUNNEL_PID 2>/dev/null
    exit
}
trap cleanup SIGINT

# Start Nodes
echo "🟢 Starting Node 1 (Bootstrap)..."
$BIN --listen /ip4/127.0.0.1/tcp/4001 --grpc-port 50051 \
     --config example/node1/config.json \
     --keypair example/node1/keypair \
     > example/node1/node.log 2>&1 &
PID_1=$!

sleep 2

echo "🟢 Starting Nodes 2-4..."
$BIN --listen /ip4/127.0.0.1/tcp/4002 --grpc-port 50052 --dial /ip4/127.0.0.1/tcp/4001 --config example/node2/config.json --keypair example/node2/keypair > example/node2/node.log 2>&1 &
PID_2=$!
$BIN --listen /ip4/127.0.0.1/tcp/4003 --grpc-port 50053 --dial /ip4/127.0.0.1/tcp/4001 --config example/node3/config.json --keypair example/node3/keypair > example/node3/node.log 2>&1 &
PID_3=$!
$BIN --listen /ip4/127.0.0.1/tcp/4004 --grpc-port 50054 --dial /ip4/127.0.0.1/tcp/4001 --config example/node4/config.json --keypair example/node4/keypair > example/node4/node.log 2>&1 &
PID_4=$!

# Start Frontends (PROD MODE - Tunnel URLs)
# Vite with --mode production uses .env.production
echo "🚀 Starting Wallet Frontend (PROD)..."
(cd atlas-wallet/frontend && npm run build && npm run preview -- --port 5173) > example/wallet_frontend.log 2>&1 &
PID_WALLET=$!

echo "🚀 Starting Explorer (PROD)..."
(cd atlas-ledger/explorer && npm run build && npm run preview -- --port 5174) > example/explorer.log 2>&1 &
PID_EXPLORER=$!

echo ""
echo "============================================="
echo "✅ PROD Cluster Iniciado!"
echo "   Wallet:   https://1961-wallet.atdigitalbank.com.br"
echo "   Explorer: https://1961-explorer.atdigitalbank.com.br"
echo "============================================="
echo "Press Ctrl+C to stop."

wait
