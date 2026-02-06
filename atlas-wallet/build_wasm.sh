#!/bin/bash
set -e

echo "🚀 Building WASM package for Atlas Wallet..."

# Ensure we are in the atlas-wallet directory
cd "$(dirname "$0")"

# Build for web target and output to frontend/src/pkg
wasm-pack build --target web --out-dir frontend/src/pkg --out-name atlas_wallet

echo "✅ WASM build successful! Package exported to frontend/src/pkg"
