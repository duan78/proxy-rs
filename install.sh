#!/bin/bash

# 🚀 Proxy.rs v0.4.0 - Script d'Installation Unifié
# Installe et configure Proxy.rs automatiquement
# Usage: curl -sSL https://raw.githubusercontent.com/duan78/proxy-rs/main/install.sh | bash

set -euo pipefail

# Configuration
PROJECT_NAME="proxy-rs"
DEPLOY_PATH="/opt/$PROJECT_NAME"
SERVICE_NAME="proxy-rs"
SERVICE_USER="proxy-rs"
CONFIG_PATH="/etc/$PROJECT_NAME"
LOG_PATH="/var/log/$PROJECT_NAME"
PROXY_PORT="${PROXY_PORT:-8080}"
API_PORT="${API_PORT:-3000}"

# Couleurs
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}🚀 Installation Proxy.rs v0.4.0${NC}"
echo "=================================="

# Vérification root
if [[ $EUID -ne 0 ]]; then
   echo -e "${RED}❌ Ce script doit être exécuté en tant que root (sudo)${NC}"
   exit 1
fi

# Mise à jour système
echo -e "${YELLOW}📦 Mise à jour du système...${NC}"
apt update && apt upgrade -y

# Installation dépendances
echo -e "${YELLOW}🔧 Installation dépendances...${NC}"
apt install -y curl wget git build-essential pkg-config libssl-dev

# Installation Rust
echo -e "${YELLOW}🦀 Installation Rust...${NC}"
if ! command -v cargo >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    export PATH="$HOME/.cargo/bin:$PATH"
    echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
fi

# Création utilisateur système
echo -e "${YELLOW}👤 Création utilisateur $SERVICE_USER...${NC}"
if ! id "$SERVICE_USER" &>/dev/null; then
    useradd -r -s /bin/false "$SERVICE_USER"
fi

# Clonage du projet
echo -e "${YELLOW}📥 Clonage du projet...${NC}"
if [[ -d "$DEPLOY_PATH" ]]; then
    rm -rf "$DEPLOY_PATH"
fi
git clone https://github.com/duan78/proxy-rs.git "$DEPLOY_PATH"

# Compilation
echo -e "${YELLOW}🔨 Compilation en mode release...${NC}"
cd "$DEPLOY_PATH"
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release

# Création répertoires
echo -e "${YELLOW}📁 Création répertoires...${NC}"
mkdir -p "$CONFIG_PATH" "$LOG_PATH"
chown -R "$SERVICE_USER:$SERVICE_USER" "$DEPLOY_PATH" "$CONFIG_PATH" "$LOG_PATH"

# Configuration simple
echo -e "${YELLOW}⚙️ Configuration...${NC}"
cat > "$CONFIG_PATH/proxy-rs.toml" << 'EOF'
[general]
max_connections = 1000
default_timeout = 8
log_level = "info"

[server]
host = "0.0.0.0"
port = 8080
max_clients = 500
client_timeout = 30

[api]
enabled = true
port = 3000
host = "0.0.0.0"
enable_cors = true
rate_limit = 100

[protocols]
http = true
https = true
socks4 = true
socks5 = true
connect_25 = false
connect_80 = true

[logging]
level = "info"
format = "json"
output = "stdout"
file_path = "/var/log/proxy-rs/proxy-rs.log"
max_file_size_mb = 50
max_files = 3
EOF

chown "$SERVICE_USER:$SERVICE_USER" "$CONFIG_PATH/proxy-rs.toml"

# Service systemd
echo -e "${YELLOW}⚙️ Configuration service systemd...${NC}"
cat > "/etc/systemd/system/$SERVICE_NAME.service" << EOF
[Unit]
Description=Proxy.rs High-Performance Proxy Server
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$SERVICE_USER
WorkingDirectory=$DEPLOY_PATH
ExecStart=$DEPLOY_PATH/target/release/proxy-rs serve --host 0.0.0.0 --port $PROXY_PORT --types HTTP HTTPS SOCKS4 SOCKS5 --max-tries 3
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal
TimeoutStartSec=60
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
EOF

# Firewall
echo -e "${YELLOW}🔥 Configuration firewall...${NC}"
ufw allow $PROXY_PORT/tcp comment "Proxy.rs Server"
ufw allow $API_PORT/tcp comment "Proxy.rs API"

# Démarrage service
echo -e "${YELLOW}🎯 Démarrage du service...${NC}"
systemctl daemon-reload
systemctl enable "$SERVICE_NAME"
systemctl restart "$SERVICE_NAME"

# Vérification
echo -e "${YELLOW}📊 Vérification installation...${NC}"
sleep 3

if systemctl is-active --quiet "$SERVICE_NAME"; then
    echo -e "${GREEN}✅ Service actif${NC}"
else
    echo -e "${RED}❌ Service inactif${NC}"
    journalctl -u "$SERVICE_NAME" --no-pager -n 10
    exit 1
fi

# Tests
echo -e "${YELLOW}🧪 Tests de fonctionnement...${NC}"
SERVER_IP=$(curl -s ifconfig.me 2>/dev/null || echo "localhost")

# Test API
if curl -s -m 5 "http://localhost:$API_PORT/api/v1/health" >/dev/null 2>&1; then
    echo -e "${GREEN}✅ API fonctionnelle${NC}"
else
    echo -e "${YELLOW}⚠️  API en cours de démarrage${NC}"
fi

# Résumé
echo -e "${BLUE}🎉 INSTALLATION TERMINÉE !${NC}"
echo "============================"
echo -e "${GREEN}✅ Serveur Proxy: http://$SERVER_IP:$PROXY_PORT${NC}"
echo -e "${GREEN}✅ API REST: http://$SERVER_IP:$API_PORT${NC}"
echo -e "${GREEN}✅ Documentation: http://$SERVER_IP:$API_PORT/docs${NC}"
echo ""
echo -e "${BLUE}Commandes utiles :${NC}"
echo "systemctl status $SERVICE_NAME           # Statut"
echo "journalctl -u $SERVICE_NAME -f          # Logs"
echo "$DEPLOY_PATH/target/release/proxy-rs --help  # Aide"
echo "$DEPLOY_PATH/target/release/proxy-rs grab --limit 10  # Test découverte"
echo ""
echo -e "${GREEN}🚀 Proxy.rs est prêt !${NC}"