#!/bin/bash

# 🚀 Test du système de judges optimisé

echo "🧪 Test des Judges Optimisés Proxy.rs"
echo "=================================="

cd "$(dirname "$0")"

# Test 1: Compilation
echo "📦 Test 1: Compilation..."
if cargo build --release --quiet; then
    echo "✅ Compilation réussie"
else
    echo "❌ Échec compilation"
    exit 1
fi

# Test 2: Help command
echo ""
echo "📋 Test 2: Vérification des commandes..."
./target/release/proxy-rs --help | head -5

# Test 3: Lancement avec logs de judges (15 secondes)
echo ""
echo "🚀 Test 3: Démarrage avec judges optimisés..."
echo "Lancement pour 15 secondes avec logs détaillés..."

timeout 15s ./target/release/proxy-rs serve --host 127.0.0.1 --port 8080 --types HTTP HTTPS --max-tries 2 2>&1 | tee judges_test.log

# Test 4: Analyse des logs
echo ""
echo "📊 Test 4: Analyse des résultats..."

if grep -q "🚀 Initialisation du système de judges optimisé" judges_test.log; then
    echo "✅ Système de judges optimisés initialisé"
else
    echo "⚠️  Pas d'initialisation des judges optimisés"
fi

if grep -q "✅ Judge disponible" judges_test.log; then
    echo "✅ Judges fonctionnels détectés"
    JUDGES_COUNT=$(grep -c "✅ Judge disponible" judges_test.log)
    echo "📈 Nombre de judges: $JUDGES_COUNT"
else
    echo "⚠️  Aucun judge fonctionnel détecté"
fi

if grep -q "🎯 Judges optimisés:" judges_test.log; then
    echo "✅ Statistiques des judges disponibles"
    grep "🎯 Judges optimisés:" judges_test.log
else
    echo "⚠️  Pas de statistiques de judges"
fi

# Test 5: Vérification des ports
echo ""
echo "🌐 Test 5: Vérification des ports..."
if command -v netstat >/dev/null 2>&1; then
    if netstat -tuln 2>/dev/null | grep -E ':(8080|3000)' > /dev/null; then
        echo "✅ Ports d'écoute détectés"
        netstat -tuln 2>/dev/null | grep -E ':(8080|3000)'
    else
        echo "❌ Aucun port d'écoute détecté"
    fi
else
    echo "⚠️  netstat non disponible"
fi

# Test 6: Test API si disponible
echo ""
echo "🔗 Test 6: Test API..."
if curl -s -m 3 http://localhost:3000/api/v1/health >/dev/null 2>&1; then
    echo "✅ API Health endpoint fonctionnel"
    curl -s http://localhost:3000/api/v1/health | head -3
else
    echo "⚠️  API non disponible ou timeout"
fi

# Nettoyage
echo ""
echo "🧹 Nettoyage..."
pkill -f proxy-rs 2>/dev/null || true
rm -f judges_test.log

echo ""
echo "🎯 Test terminé !"
echo ""
echo "📊 Résumé des performances attendues:"
echo "- ⚡ Judges ultra-rapides (< 500ms)"
echo "- 🔄 Pool de clients HTTP réutilisables"
echo "- 📈 Load balancing automatique"
echo "- 🛡️ Validation haute performance"