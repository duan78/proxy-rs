#!/bin/bash

# 🚀 Quick Start Proxy.rs v0.4.0
# Démarrage rapide pour test et développement

echo "🚀 Proxy.rs Quick Start"
echo "======================="

# Vérifier si le binaire existe
if [[ ! -f "./target/release/proxy-rs" ]]; then
    echo "📦 Compilation en cours..."
    cargo build --release
fi

# Démarrer le serveur avec configuration par défaut
echo "🌐 Démarrage du serveur proxy sur http://localhost:8080"
echo "📊 API disponible sur http://localhost:3000"
echo "📚 Documentation: http://localhost:3000/docs"
echo ""
echo "Ctrl+C pour arrêter"
echo ""

./target/release/proxy-rs serve --host 127.0.0.1 --port 8080 --types HTTP HTTPS SOCKS4 SOCKS5