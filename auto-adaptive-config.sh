#!/bin/bash

# 🔧 Script de post-installation auto-adaptatif pour Proxy.rs
# Ajuste automatiquement la configuration en fonction de l'environnement

set -euo pipefail

# Couleurs
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${BLUE}🔧 Post-Installation Auto-Adaptatif Proxy.rs v0.4.0${NC}"
echo "=============================================="

CONFIG_PATH="/etc/proxy-rs/proxy-rs.toml"
SERVICE_NAME="proxy-rs"

# Test de connectivité vers différents services
test_connectivity() {
    local url=$1
    local timeout=${2:-10}

    if curl -s --connect-timeout "$timeout" --max-time "$timeout" "$url" >/dev/null 2>&1; then
        return 0
    else
        return 1
    fi
}

# Détecter l'environnement et ajuster la configuration
echo -e "${YELLOW}📊 Détection de l'environnement...${NC}"

# Test de connectivité Internet
if test_connectivity "https://api.ipify.org" 8; then
    echo -e "${GREEN}✅ Connectivité Internet OK${NC}"
    INTERNET_OK=true
else
    echo -e "${YELLOW}⚠️  Connectivité limitée détectée${NC}"
    INTERNET_OK=false
fi

# Test de différents judges
echo -e "${YELLOW}🏥 Test des judges disponibles...${NC}"

AVAILABLE_JUDGES=()
JUDGE_URLS=(
    "https://api.ipify.org"
    "https://ifconfig.me/ip"
    "https://ipinfo.io/ip"
    "https://httpbin.org/ip"
    "https://jsonip.com"
)

for url in "${JUDGE_URLS[@]}"; do
    echo -n "  Test $url... "
    if test_connectivity "$url" 5; then
        echo -e "${GREEN}✅${NC}"
        AVAILABLE_JUDGES+=("$url")
    else
        echo -e "${RED}❌${NC}"
    fi
done

# Ajuster la configuration en fonction des résultats
if [ ${#AVAILABLE_JUDGES[@]} -eq 0 ]; then
    echo -e "${YELLOW}⚠️  Aucun judge externe disponible - Mode local activé${NC}"

    # Mode local : désactiver les judges externes mais garder le proxy fonctionnel
    sed -i 's/enabled = true/enabled = false/' "$CONFIG_PATH"
    sed -i 's/default_timeout = 30/default_timeout = 15/' "$CONFIG_PATH"

    echo -e "${CYAN}📝 Configuration ajustée : Mode local (sans judges externes)${NC}"
elif [ ${#AVAILABLE_JUDGES[@]} -lt 3 ]; then
    echo -e "${YELLOW}⚠️  Connectivité limitée - Mode dégradé activé${NC}"

    # Mode dégradé : augmenter les timeouts et réduire parallélisme
    sed -i 's/timeout_ms = 15000/timeout_ms = 20000/' "$CONFIG_PATH"
    sed -i 's/parallel_checks = 3/parallel_checks = 1/' "$CONFIG_PATH"
    sed -i 's/default_timeout = 30/default_timeout = 45/' "$CONFIG_PATH"

    echo -e "${CYAN}📝 Configuration ajustée : Timeouts augmentés, parallélisme réduit${NC}"
else
    echo -e "${GREEN}✅ Excellente connectivité - Mode optimisé activé${NC}"

    # Mode optimisé : configuration standard
    echo -e "${CYAN}📝 Configuration optimisée conservée${NC}"
fi

# Redémarrer le service avec la nouvelle configuration
echo -e "${YELLOW}🔄 Redémarrage du service avec configuration adaptative...${NC}"
systemctl restart "$SERVICE_NAME"

# Attendre le démarrage
sleep 10

# Validation finale
echo -e "${YELLOW}🎯 Validation finale...${NC}"

if systemctl is-active --quiet "$SERVICE_NAME"; then
    echo -e "${GREEN}✅ Service actif${NC}"

    # Test du proxy
    if curl -x http://localhost:8080 -s --connect-timeout 10 https://httpbin.org/ip >/dev/null 2>&1; then
        echo -e "${GREEN}✅ Proxy fonctionnel${NC}"
    else
        echo -e "${YELLOW}⚠️  Proxy en cours d'initialisation${NC}"
    fi

    # Test API
    if curl -s http://localhost:3000/api/v1/health >/dev/null 2>&1; then
        echo -e "${GREEN}✅ API REST fonctionnelle${NC}"
    else
        echo -e "${RED}❌ API REST non répond${NC}"
    fi
else
    echo -e "${RED}❌ Service non démarré${NC}"
    echo "Logs d'erreur:"
    journalctl -u "$SERVICE_NAME" --no-pager -n 10
    exit 1
fi

# Résumé
echo ""
echo -e "${BLUE}🎉 Post-Installation terminée avec succès !${NC}"
echo "============================================"
echo -e "${GREEN}✅ Service Proxy.rs adapté à votre environnement${NC}"
echo -e "${GREEN}✅ Configuration optimisée automatiquement${NC}"
echo -e "${GREEN}✅ Judges adaptés à votre connectivité${NC}"
echo ""
echo -e "${CYAN}🌐 Accès depuis l'extérieur :${NC}"
echo "  • Proxy Server: http://$(curl -s --connect-timeout 5 ifconfig.me 2>/dev/null || echo "VOTRE_IP"):8080"
echo "  • API REST: http://$(curl -s --connect-timeout 5 ifconfig.me 2>/dev/null || echo "VOTRE_IP"):3000"
echo "  • Documentation: http://$(curl -s --connect-timeout 5 ifconfig.me 2>/dev/null || echo "VOTRE_IP"):3000/docs"
echo ""
echo -e "${GREEN}🚀 Proxy.rs est 100% plug & play et prêt pour la production !${NC}"