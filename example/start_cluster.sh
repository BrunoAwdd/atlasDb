#!/bin/bash

# Load NVM for npm access (needed when running from desktop shortcut)
export NVM_DIR="$HOME/.nvm"
[ -s "$NVM_DIR/nvm.sh" ] && \. "$NVM_DIR/nvm.sh"

export RUST_LOG=info
# Prevent Segfaults by using Release mode (Optimized Stack/Heap usage)
echo "🚀 Starting AtlasDB Cluster with 4 nodes..."

# 1. Build in Release Mode (Robustness)
echo "🔨 Building atlas-node (Release Mode)..."
cargo build -p atlas-node --bin atlas-node --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

BIN="./target/release/atlas-node"

# 2. Cleanup Function
cleanup() {
    echo "🛑 Stopping Cluster..."
    kill $PID_1 $PID_2 $PID_3 $PID_4 $PID_WALLET $PID_EXPLORER 2>/dev/null
    exit
}
trap cleanup SIGINT

# 3. Start Nodes
# Node 1 (Bootstrap)
echo "🟢 Starting Node 1 (Bootstrap)..."
$BIN --listen /ip4/127.0.0.1/tcp/4001 --grpc-port 50051 \
     --config example/node1/config.json \
     --keypair example/node1/keypair \
     > example/node1/node.log 2>&1 &
PID_1=$!
echo "Node 1 PID: $PID_1"

sleep 2

# Node 2
echo "🟢 Starting Node 2..."
$BIN --listen /ip4/127.0.0.1/tcp/4002 --grpc-port 50052 \
     --dial /ip4/127.0.0.1/tcp/4001 \
     --config example/node2/config.json \
     --keypair example/node2/keypair \
     > example/node2/node.log 2>&1 &
PID_2=$!
echo "Node 2 PID: $PID_2"

# Node 3
echo "🟢 Starting Node 3..."
$BIN --listen /ip4/127.0.0.1/tcp/4003 --grpc-port 50053 \
     --dial /ip4/127.0.0.1/tcp/4001 \
     --config example/node3/config.json \
     --keypair example/node3/keypair \
     > example/node3/node.log 2>&1 &
PID_3=$!
echo "Node 3 PID: $PID_3"

# Node 4
echo "🟢 Starting Node 4..."
$BIN --listen /ip4/127.0.0.1/tcp/4004 --grpc-port 50054 \
     --dial /ip4/127.0.0.1/tcp/4001 \
     --config example/node4/config.json \
     --keypair example/node4/keypair \
     > example/node4/node.log 2>&1 &
PID_4=$!
echo "Node 4 PID: $PID_4"

# 4. Start Frontends
echo "🚀 Starting Atlas Wallet Frontend..."
(cd atlas-wallet/frontend && npm run dev:tunnel) > example/wallet_frontend.log 2>&1 &
PID_WALLET=$!
echo "Wallet PID: $PID_WALLET"

echo "🚀 Starting Atlas Ledger Explorer..."
(cd atlas-ledger/explorer && npm run dev:tunnel) > example/explorer.log 2>&1 &
PID_EXPLORER=$!
echo "Explorer PID: $PID_EXPLORER"



echo "✅ Cluster started! Logs are in example/node*/node.log"
echo "Press Ctrl+C to stop all nodes."

wait
