#!/bin/bash
set -e

echo "🪟 Preparando para compilar AtlasDB para Windows (x64)..."

# 1. Verifica dependências do Arch Linux
if ! command -v x86_64-w64-mingw32-gcc &> /dev/null; then
    echo "❌ Compilador MinGW não encontrado!"
    echo "👉 Instale com: sudo pacman -S mingw-w64-gcc"
    exit 1
fi

# 2. Adiciona o target no Rust
echo "🛠️ Adicionando target x86_64-pc-windows-gnu..."
rustup target add x86_64-pc-windows-gnu

# 3. Compila (configurando variáveis para rocksdb C++)
# Limpa TUDO para garantir reconstrução
cargo clean

# Garante que o Rust seja generico (boa pratica para Windows antigos)
export RUSTFLAGS="-C target-cpu=x86-64"

echo "🚀 Compilando atlas-node (com Redb)..."
cargo build --release --target x86_64-pc-windows-gnu -p atlas-node

echo "📦 Empacotando DLLs básicas do MinGW..."
TARGET_DIR="target/x86_64-pc-windows-gnu/release"
MINGW_BIN="/usr/x86_64-w64-mingw32/bin"

cp "$MINGW_BIN/libstdc++-6.dll" "$TARGET_DIR/"
cp "$MINGW_BIN/libgcc_s_seh-1.dll" "$TARGET_DIR/"
cp "$MINGW_BIN/libwinpthread-1.dll" "$TARGET_DIR/"

echo "✅ Compilação Concluída!"
echo "📂 Pasta de saída: $TARGET_DIR"
echo "👉 Jogue O CONTEÚDO DESSA PASTA (exe + dlls) para os PCs Windows."
echo "   (Voce precisa levar as DLLs junto com o .exe!)"
