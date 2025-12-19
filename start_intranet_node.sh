#!/bin/bash

# Usage: ./start_intranet_node.sh <MY_IP> [PEER_ID_TO_DIAL] [PEER_IP_TO_DIAL]

MY_IP=$1
DIAL_PEER_ID=$2
DIAL_PEER_IP=$3

if [ -z "$MY_IP" ]; then
  echo "Usage: ./start_intranet_node.sh <MY_IP> [PEER_ID_TO_DIAL] [PEER_IP_TO_DIAL]"
  echo "Example (Seed): ./start_intranet_node.sh 192.168.1.10"
  echo "Example (Peer): ./start_intranet_node.sh 192.168.1.11 12D3Koo... 192.168.1.10"
  exit 1
fi

# Ensure keys dir exists
mkdir -p keys

# Generate a unique config name based on IP/random to avoid conflicts if testing locally
NODE_NAME="node-$(echo $MY_IP | awk -F. '{print $4}')"
CONFIG_FILE="${NODE_NAME}-config.json"
KEYPAIR_PATH="keys/${NODE_NAME}-key"

CMD="cargo run -p atlas-node --bin atlas-node --release -- --config $CONFIG_FILE --listen /ip4/$MY_IP/tcp/4001 --grpc-port 50051 --keypair $KEYPAIR_PATH"

if [ ! -z "$DIAL_PEER_ID" ] && [ ! -z "$DIAL_PEER_IP" ]; then
  DIAL_ADDR="/ip4/$DIAL_PEER_IP/tcp/4001/p2p/$DIAL_PEER_ID"
  echo "🚀 Starting Node ($MY_IP) connecting to $DIAL_ADDR..."
  $CMD --dial "$DIAL_ADDR"
else
  echo "🌱 Starting Seed Node ($MY_IP)..."
  $CMD
fi
