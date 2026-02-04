#!/bin/bash

# Script de Suporte - Início Rápido (AtlasDB)
# Executa o Tunnel Cloudflare e Inicia o Cluster

echo "============================================="
echo "🚀 INICIANDO MODO SUPORTE ATLAS DB"
echo "============================================="

# 1. Iniciar Cloudflare Tunnel
echo "🌐 Conectando Cloudflare Tunnel..."
if pgrep -x "cloudflared" > /dev/null
then
    echo "⚠️  Tunnel já parece estar rodando. Ignorando start."
else
    cloudflared tunnel run at-tunnel > tunnel.log 2>&1 &
    TUNNEL_PID=$!
    echo "✅ Tunnel iniciado (PID: $TUNNEL_PID)"
fi

# 2. Cleanup Function (Garante que o tunnel morra se cancelarmos o script)
cleanup() {
    echo ""
    echo "🛑 Parando Tunnel e Encerrando..."
    if [ ! -z "$TUNNEL_PID" ]; then
        kill $TUNNEL_PID 2>/dev/null
    fi
    exit
}
trap cleanup SIGINT

# Espera um pouco para garantir conexão
sleep 3

# 3. Navegar e Limpar Dados
# Garantir que estamos no diretório certo (supondo execução da home ou do folder)
DIR="/home/bruno/projects/atlasDb"
if [ -d "$DIR" ]; then
    cd "$DIR"
else
    echo "❌ Diretório $DIR não encontrado!"
    cleanup
fi

echo "🧹 Limpando dados antigos (Reset)..."
chmod +x clean_data.sh
./clean_data.sh

# 4. Iniciar Cluster (Este comando "segura" o terminal até Ctrl+C)
echo "🔥 Iniciando Cluster Atlas..."
chmod +x example/start_cluster.sh
./example/start_cluster.sh

# Quando o start_cluster terminar (Ctrl+C lá dentro), executamos cleanup
cleanup
