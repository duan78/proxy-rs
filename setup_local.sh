#!/bin/bash

# Script d'installation locale Proxy.rs v0.4.0
# À exécuter DIRECTEMENT sur le VPS (pas de SSH)
# Usage: sudo bash setup_local.sh

set -euo pipefail

# Configuration
PROJECT_NAME="proxy-rs"
DEPLOY_PATH="/opt/$PROJECT_NAME"
SERVICE_NAME="proxy-rs"
SERVICE_USER="proxy-rs"
CONFIG_PATH="/etc/$PROJECT_NAME"
LOG_PATH="/var/log/$PROJECT_NAME"
LIB_PATH="/var/lib/$PROJECT_NAME"
MAX_CONNECTIONS="${MAX_CONNECTIONS:-5000}"
PROXY_PORT="${PROXY_PORT:-8080}"
API_PORT="${API_PORT:-3000}"

# Couleurs pour l'affichage
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🚀 Installation Locale Proxy.rs v0.4.0 sur VPS${NC}"
echo "================================================="

# Vérification des privilèges
if [[ $EUID -ne 0 ]]; then
   echo -e "${RED}❌ Ce script doit être exécuté en tant que root (sudo)${NC}"
   exit 1
fi

# Vérification qu'on est bien dans le répertoire du projet
if [[ ! -f "Cargo.toml" ]]; then
    echo -e "${RED}❌ Ce script doit être exécuté depuis le répertoire racine de Proxy.rs (où se trouve Cargo.toml)${NC}"
    echo "Exemple: cd /opt/proxy-rs && sudo bash setup_local.sh"
    exit 1
fi

echo -e "${GREEN}✅ Répertoire du projet détecté: $(pwd)${NC}"

# Mise à jour système et installation dépendances
echo -e "${YELLOW}🔧 Mise à jour système et installation dépendances...${NC}"
apt update && apt upgrade -y
apt install -y build-essential pkg-config libssl-dev git ufw htop curl wget netcat-openbsd systemd

# Installation Rust si nécessaire
echo -e "${YELLOW}🦀 Installation de Rust...${NC}"
if ! command -v cargo >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    export PATH="$HOME/.cargo/bin:$PATH"
    echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
fi

rustc --version
cargo --version

# Création utilisateur système dédié
echo -e "${YELLOW}👤 Création utilisateur système $SERVICE_USER...${NC}"
if ! id "$SERVICE_USER" &>/dev/null; then
    useradd -r -s /bin/false "$SERVICE_USER"
    usermod -L "$SERVICE_USER"
    echo -e "${GREEN}✅ Utilisateur $SERVICE_USER créé${NC}"
else
    echo -e "${YELLOW}ℹ️  Utilisateur $SERVICE_USER existe déjà${NC}"
fi

# Création répertoires avec permissions
echo -e "${YELLOW}📁 Création des répertoires de l'application...${NC}"
mkdir -p "$DEPLOY_PATH" "$CONFIG_PATH" "$LOG_PATH" "$LIB_PATH"
chown -R "$SERVICE_USER:$SERVICE_USER" "$DEPLOY_PATH" "$CONFIG_PATH" "$LOG_PATH" "$LIB_PATH"
chmod 755 "$DEPLOY_PATH" "$CONFIG_PATH"
chmod 750 "$LOG_PATH" "$LIB_PATH"

# Copie des fichiers du projet vers /opt
echo -e "${YELLOW}📦 Copie des fichiers du projet...${NC}"
cp -r ./* "$DEPLOY_PATH/"
chown -R "$SERVICE_USER:$SERVICE_USER" "$DEPLOY_PATH"

# Configuration production optimisée
echo -e "${YELLOW}⚙️ Création configuration production...${NC}"
cat > "$CONFIG_PATH/proxy-rs.toml" << CONFIG_EOF
# ===========================================
# CONFIGURATION PRODUCTION PROXY.RS v0.4.0
# ===========================================

[general]
max_connections = $MAX_CONNECTIONS
default_timeout = 8
log_level = "warn"
enable_metrics = true
max_concurrent_checks = 5000
cleanup_interval = 300
memory_limit_mb = 500
max_avg_response_time_ms = 2000
min_requests_for_filtering = 5

[server]
host = "0.0.0.0"
port = $PROXY_PORT
max_clients = 2000
client_timeout = 30
enable_keep_alive = true

[api]
enabled = true
port = $API_PORT
host = "0.0.0.0"
enable_cors = true
rate_limit = 1000
enable_auth = false

[dnsbl]
enabled = true
timeout_secs = 5
max_concurrent = 10
cache_ttl_secs = 3600
malicious_threshold = 2

[protocols]
http = true
https = true
socks4 = true
socks5 = true
connect_25 = false
connect_80 = true

[geolocation]
enabled = true
auto_update = true
update_interval_hours = 168
allowed_countries = ""
excluded_countries = "CN,RU,KP"

[performance]
enable_connection_pooling = true
pool_size = 200
enable_pipelining = true
compression_enabled = true
l1_cache_size = 1000
l2_cache_size = 10000
cache_ttl = 300

[logging]
level = "warn"
format = "json"
output = "stdout"
file_path = "$LOG_PATH/proxy-rs.log"
max_file_size_mb = 100
max_files = 5
CONFIG_EOF

chown "$SERVICE_USER:$SERVICE_USER" "$CONFIG_PATH/proxy-rs.toml"
chmod 640 "$CONFIG_PATH/proxy-rs.toml"

# Compilation en mode release
echo -e "${YELLOW}🔨 Compilation en mode release optimisé...${NC}"
cd "$DEPLOY_PATH"
export PATH="$HOME/.cargo/bin:$PATH"

# Nettoyage compilation précédente
echo "Nettoyage compilation précédente..."
cargo clean

# Compilation avec optimisations
echo "Compilation avec optimisations..."
CARGO_TARGET_DIR="$DEPLOY_PATH/target" cargo build --release

# Vérification du binaire
if [[ ! -f "target/release/proxy-rs" ]]; then
    echo -e "${RED}❌ Erreur de compilation - binaire non trouvé${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Compilation terminée avec succès${NC}"
ls -la target/release/proxy-rs

# Copie du binaire à la racine du projet
cp target/release/proxy-rs .
chmod +x proxy-rs
chown "$SERVICE_USER:$SERVICE_USER" proxy-rs

# Service systemd avec sécurité renforcée
echo -e "${YELLOW}⚙️ Création service systemd sécurisé...${NC}"
cat > "/etc/systemd/system/$SERVICE_NAME.service" << SERVICE_EOF
[Unit]
Description=Proxy.rs High-Performance Proxy Server v0.4.0
After=network.target network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
WorkingDirectory=$DEPLOY_PATH
ExecStart=$DEPLOY_PATH/proxy-rs serve --config $CONFIG_PATH/proxy-rs.toml
ExecReload=/bin/kill -HUP \$MAINPID
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=proxy-rs

# Sécurité renforcée
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$LOG_PATH $LIB_PATH $DEPLOY_PATH
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictRealtime=true
RestrictSUIDSGID=true
RemoveIPC=true
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
SERVICE_EOF

chmod 644 "/etc/systemd/system/$SERVICE_NAME.service"

# Configuration firewall avancée
echo -e "${YELLOW}🔥 Configuration firewall UFW...${NC}"
ufw --force reset
ufw default deny incoming
ufw default allow outgoing
ufw allow 22/tcp comment 'SSH Access'
ufw allow $PROXY_PORT/tcp comment 'Proxy.rs Server'
ufw allow $API_PORT/tcp comment 'Proxy.rs API'
ufw --force enable
ufw status verbose

# Activation et démarrage du service
echo -e "${YELLOW}🎯 Activation et démarrage du service...${NC}"
systemctl daemon-reload
systemctl enable "$SERVICE_NAME"
systemctl restart "$SERVICE_NAME"

# Attendre démarrage du service
echo "Attente démarrage du service..."
sleep 5

# Vérification statut
if systemctl is-active --quiet "$SERVICE_NAME"; then
    echo -e "${GREEN}✅ Service actif${NC}"
else
    echo -e "${RED}❌ Service inactif${NC}"
    systemctl status "$SERVICE_NAME" --no-pager
    journalctl -u "$SERVICE_NAME" --no-pager -n 20
    exit 1
fi

# Vérifications post-installation
echo -e "${BLUE}📊 Vérifications post-installation${NC}"
echo "=================================="

# Statut du service
echo -e "${YELLOW}🔍 Statut du service:${NC}"
systemctl status "$SERVICE_NAME" --no-pager -l

# Vérification des ports
echo -e "${YELLOW}🌐 Vérification des ports d'écoute:${NC}"
netstat -tuln | grep -E ':(8080|3000)' || echo 'Ports pas encore en écoute'

# Test API Health
echo -e "${YELLOW}🏥 Test API Health:${NC}"
SERVER_IP=$(curl -s ifconfig.me 2>/dev/null || echo "localhost")
if curl -s -m 10 "http://localhost:$API_PORT/api/v1/health" >/dev/null; then
    echo -e "${GREEN}✅ API Health OK${NC}"
else
    echo -e "${YELLOW}⚠️  API Health en cours de démarrage${NC}"
fi

# Test proxy rotation
echo -e "${YELLOW}🔄 Test proxy rotation:${NC}"
if timeout 10s curl -x "http://localhost:$PROXY_PORT" -s "https://httpbin.org/ip" >/dev/null; then
    echo -e "${GREEN}✅ Proxy rotation OK${NC}"
else
    echo -e "${YELLOW}⚠️  Proxy rotation en cours d'initialisation${NC}"
fi

# Logs récents
echo -e "${YELLOW}📝 Logs récents du service:${NC}"
journalctl -u "$SERVICE_NAME" --no-pager -n 5

# Résumé de l'installation
echo -e "${BLUE}🎉 INSTALLATION TERMINÉE AVEC SUCCÈS !${NC}"
echo "================================================="
echo -e "${GREEN}✅ Serveur Proxy: http://$SERVER_IP:$PROXY_PORT${NC}"
echo -e "${GREEN}✅ API REST: http://$SERVER_IP:$API_PORT${NC}"
echo -e "${GREEN}✅ Documentation: http://$SERVER_IP:$API_PORT/docs${NC}"
echo -e "${GREEN}✅ Health Check: http://$SERVER_IP:$API_PORT/api/v1/health${NC}"
echo ""
echo -e "${BLUE}📊 Commandes utiles :${NC}"
echo "systemctl status $SERVICE_NAME                    # Statut du service"
echo "journalctl -u $SERVICE_NAME -f                   # Logs en temps réel"
echo "curl http://localhost:$API_PORT/api/v1/metrics   # Métriques"
echo "$DEPLOY_PATH/proxy-rs --help                     # Aide commandes"
echo "$DEPLOY_PATH/proxy-rs grab --limit 10            # Test découverte"
echo ""
echo -e "${GREEN}🚀 Proxy.rs v0.4.0 est maintenant installé et fonctionnel !${NC}"