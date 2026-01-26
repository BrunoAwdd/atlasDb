#!/bin/bash


echo "🛑 Stopping any running nodes..."
pkill -f atlas-node || true
sleep 1

echo "🧹 Cleaning node data directories..."

rm -rf example/node*/data
rm -rf example/node*/node.log
rm -rf audits
rm -rf logs

echo "✨ Done! All data and audits cleared."
