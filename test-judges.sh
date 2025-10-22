#!/bin/bash

# 🧪 Script de test pour diagnostiquer les problèmes de judges

echo "🧪 Test des Judges Proxy.rs"
echo "=========================="

cd /opt/proxy-rs

# Test 1: Vérifier les arguments de la commande serve
echo "📋 Test 1: Arguments de la commande serve"
echo "Commande: ./target/release/proxy-rs serve --help"
./target/release/proxy-rs serve --help | head -20
echo ""

# Test 2: Démarrage avec logs détaillés
echo "📋 Test 2: Démarrage avec logs détaillés"
echo "Lancement du serveur pour 15 secondes avec logs..."
timeout 15s ./target/release/proxy-rs serve --host 0.0.0.0 --port 8080 --types HTTP --max-tries 1 2>&1 || true
echo ""

# Test 3: Test avec différents types de protocoles
echo "📋 Test 3: Test avec différents protocoles"
echo "Test avec HTTP uniquement..."
timeout 10s ./target/release/proxy-rs serve --host 0.0.0.0 --port 8081 --types HTTP --max-tries 1 2>&1 || true
echo ""

# Test 4: Vérifier si les ports répondent
echo "📋 Test 4: Vérification des ports"
if pgrep -f "proxy-rs serve" > /dev/null; then
    echo "✅ Processus proxy-rs trouvé"
    netstat -tuln | grep -E ':(8080|8081|3000)' || echo "❌ Aucun port trouvé"
else
    echo "❌ Aucun processus proxy-rs trouvé"
fi

echo ""
echo "🎯 Test terminé !"