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
echo "🚀 Compilando atlas-node... (Isso pode demorar porque compila o RocksDB do zero)"

# Limpa TUDO para garantir reconstrução sem flags antigas
cargo clean

# Garante que o RocksDB nao use instrucoes AVX/SSE recentes que quebram em CPUs antigas
export ROCKSDB_PORTABLE=1
# Garante que o Rust tambem seja generico
export RUSTFLAGS="-C target-cpu=x86-64"

cargo build --release --target x86_64-pc-windows-gnu -p atlas-node

echo "📦 Empacotando DLLs necessárias..."
TARGET_DIR="target/x86_64-pc-windows-gnu/release"
MINGW_BIN="/usr/x86_64-w64-mingw32/bin"

cp "$MINGW_BIN/libstdc++-6.dll" "$TARGET_DIR/"
cp "$MINGW_BIN/libgcc_s_seh-1.dll" "$TARGET_DIR/"
cp "$MINGW_BIN/libwinpthread-1.dll" "$TARGET_DIR/"
# Extras que o RocksDB pode pedir
cp "$MINGW_BIN/libssp-0.dll" "$TARGET_DIR/" 2>/dev/null || :
cp "$MINGW_BIN/libatomic-1.dll" "$TARGET_DIR/" 2>/dev/null || :
cp "$MINGW_BIN/libgomp-1.dll" "$TARGET_DIR/" 2>/dev/null || :
cp "$MINGW_BIN/libquadmath-0.dll" "$TARGET_DIR/" 2>/dev/null || :

echo "✅ Compilação Concluída!"
echo "📂 Pasta de saída: $TARGET_DIR"
echo "👉 Jogue O CONTEÚDO DESSA PASTA (exe + dlls) para os PCs Windows."
echo "   (Voce precisa levar as DLLs junto com o .exe!)"
