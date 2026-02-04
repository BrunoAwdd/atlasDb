#!/bin/bash

# Script de Suporte - MODO DEV (AtlasDB)
# Executa o Cluster com URLs localhost

echo "============================================="
echo "🛠️  INICIANDO MODO DEV - ATLAS DB"
echo "============================================="

# Load NVM for npm access
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

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
    kill $PID_1 $PID_2 $PID_3 $PID_4 $PID_WALLET $PID_EXPLORER 2>/dev/null
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

# Start Frontends (DEV MODE - localhost)
echo "🚀 Starting Wallet Frontend (DEV)..."
(cd atlas-wallet/frontend && npm run dev) > example/wallet_frontend.log 2>&1 &
PID_WALLET=$!

echo "🚀 Starting Explorer (DEV)..."
(cd atlas-ledger/explorer && npm run dev) > example/explorer.log 2>&1 &
PID_EXPLORER=$!

echo ""
echo "============================================="
echo "✅ DEV Cluster Iniciado!"
echo "   Wallet:   http://localhost:5173"
echo "   Explorer: http://localhost:5174"
echo "   Node API: http://localhost:3001"
echo "============================================="
echo "Press Ctrl+C to stop."

wait
