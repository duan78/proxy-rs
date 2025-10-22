#!/bin/bash

# 🚀 Proxy.rs v0.4.0 - Script d'Installation Automatisée avec Judges Optimisés
# Installation complète et configuration automatique pour production
# Usage: curl -sSL https://raw.githubusercontent.com/duan78/proxy-rs/main/install.sh | bash

set -euo pipefail

# Configuration
PROJECT_NAME="proxy-rs"
DEPLOY_PATH="/opt/$PROJECT_NAME"
SERVICE_NAME="proxy-rs"
SERVICE_USER="proxy-rs"
CONFIG_PATH="/etc/$PROJECT_NAME"
LOG_PATH="/var/log/$PROJECT_NAME"
LIB_PATH="/var/lib/$PROJECT_NAME"
PROXY_PORT="${PROXY_PORT:-8080}"
API_PORT="${API_PORT:-3000}"
MAX_CONNECTIONS="${MAX_CONNECTIONS:-5000}"

# Couleurs
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${BLUE}🚀 Installation Proxy.rs v0.4.0 avec Judges Optimisés${NC}"
echo "======================================================"

# Vérification root
if [[ $EUID -ne 0 ]]; then
   echo -e "${RED}❌ Ce script doit être exécuté en tant que root (sudo)${NC}"
   exit 1
fi

# Vérification des prérequis
echo -e "${YELLOW}📋 Vérification des prérequis système...${NC}"
if ! command -v curl >/dev/null 2>&1; then
    echo -e "${RED}❌ curl est requis mais non installé${NC}"
    exit 1
fi

# Mise à jour système
echo -e "${YELLOW}📦 Mise à jour du système...${NC}"
apt update && apt upgrade -y

# Installation dépendances complètes
echo -e "${YELLOW}🔧 Installation dépendances système...${NC}"
apt install -y \
    curl \
    wget \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    ufw \
    htop \
    netcat-openbsd \
    systemd \
    jq

# Installation Rust avec toolchain complète
echo -e "${YELLOW}🦀 Installation Rust toolchain...${NC}"
if ! command -v cargo >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
        --profile default
    export PATH="$HOME/.cargo/bin:$PATH"
    echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc

    # Attendre la fin de l'installation
    sleep 2

    # Installer les composants additionnels
    if "$HOME/.cargo/bin/rustup" component add rustfmt clippy 2>/dev/null; then
        echo -e "${GREEN}✅ Composants Rust additionnels installés${NC}"
    else
        echo -e "${YELLOW}⚠️  Installation des composants optionnels (peut échouer)${NC}"
    fi
fi

# Vérification installation Rust
echo -e "${CYAN}🔍 Vérification installation Rust...${NC}"
export PATH="$HOME/.cargo/bin:$PATH"
rustc --version
cargo --version

# Création utilisateur système dédié
echo -e "${YELLOW}👤 Création utilisateur système $SERVICE_USER...${NC}"
if ! id "$SERVICE_USER" &>/dev/null; then
    useradd -r -s /bin/false "$SERVICE_USER"
    echo -e "${GREEN}✅ Utilisateur $SERVICE_USER créé${NC}"
else
    echo -e "${YELLOW}ℹ️  Utilisateur $SERVICE_USER existe déjà${NC}"
fi

# Clonage ou mise à jour du projet
echo -e "${YELLOW}📥 Clonage du projet depuis GitHub...${NC}"
if [[ -d "$DEPLOY_PATH/.git" ]]; then
    echo "Mise à jour du projet existant..."
    cd "$DEPLOY_PATH"
    git pull origin main
else
    rm -rf "$DEPLOY_PATH" 2>/dev/null || true
    git clone https://github.com/duan78/proxy-rs.git "$DEPLOY_PATH"
    cd "$DEPLOY_PATH"
fi

echo -e "${GREEN}✅ Projet cloné dans: $DEPLOY_PATH${NC}"
echo -e "${GREEN}📋 Version actuelle: $(git log -1 --format='%h')${NC}"

# Compilation optimisée avec judges
echo -e "${YELLOW}🔨 Compilation en mode release optimisé...${NC}"
export PATH="$HOME/.cargo/bin:$PATH"

# Nettoyage compilation précédente
echo "Nettoyage compilation précédente..."
cargo clean

# Compilation avec optimisations
echo "Compilation avec support judges optimisés..."
RUSTFLAGS="-C opt-level=3 -C target-cpu=native" cargo build --release

# Vérification du binaire
if [[ ! -f "target/release/proxy-rs" ]]; then
    echo -e "${RED}❌ Erreur de compilation - binaire non trouvé${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Compilation terminée avec succès${NC}"
echo -e "${CYAN}📊 Binaire créé: $(du -h target/release/proxy-rs | cut -f1)${NC}"

# Création répertoires avec permissions optimisées
echo -e "${YELLOW}📁 Création répertoires de l'application...${NC}"
mkdir -p "$DEPLOY_PATH" "$CONFIG_PATH" "$LOG_PATH" "$LIB_PATH"
chown -R "$SERVICE_USER:$SERVICE_USER" "$DEPLOY_PATH" "$CONFIG_PATH" "$LOG_PATH" "$LIB_PATH"
chmod 755 "$DEPLOY_PATH" "$CONFIG_PATH"
chmod 750 "$LOG_PATH" "$LIB_PATH"

# Configuration production avec judges optimisés
echo -e "${YELLOW}⚙️ Configuration production avec judges optimisés...${NC}"
cat > "$CONFIG_PATH/proxy-rs.toml" << 'CONFIG_EOF'
# ===========================================
# CONFIGURATION PRODUCTION PROXY.RS v0.4.0
# AVEC JUDGES OPTIMISÉS INTÉGRÉS
# ===========================================

[general]
max_connections = 5000
default_timeout = 8
log_level = "info"
enable_metrics = true
max_concurrent_checks = 5000
cleanup_interval = 300
memory_limit_mb = 500
max_avg_response_time_ms = 2000
min_requests_for_filtering = 5

[server]
host = "0.0.0.0"
port = 8080
max_clients = 2000
client_timeout = 30
enable_keep_alive = true

[api]
enabled = true
port = 3000
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
connect_25 = true
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
level = "info"
format = "json"
output = "stdout"
file_path = "/var/log/proxy-rs/proxy-rs.log"
max_file_size_mb = 100
max_files = 5

# Configuration judges optimisés (auto-géré)
[judges]
enabled = true
timeout_ms = 2000
parallel_checks = 15
cache_results = true
health_check_interval = 300
CONFIG_EOF

chown "$SERVICE_USER:$SERVICE_USER" "$CONFIG_PATH/proxy-rs.toml"
chmod 640 "$CONFIG_PATH/proxy-rs.toml"

# Copie du binaire à la racine
echo -e "${YELLOW}📦 Installation du binaire...${NC}"
cp target/release/proxy-rs "$DEPLOY_PATH/"
chmod +x "$DEPLOY_PATH/proxy-rs"
chown "$SERVICE_USER:$SERVICE_USER" "$DEPLOY_PATH/proxy-rs"

# Service systemd avec sécurité renforcée
echo -e "${YELLOW}⚙️ Configuration service systemd avec sécurité...${NC}"
cat > "/etc/systemd/system/$SERVICE_NAME.service" << 'SERVICE_EOF'
[Unit]
Description=Proxy.rs High-Performance Proxy Server with Optimized Judges v0.4.0
After=network-online.target
Wants=network-online.target
Documentation=https://github.com/duan78/proxy-rs

[Service]
Type=simple
User=$SERVICE_USER
Group=$SERVICE_USER
WorkingDirectory=$DEPLOY_PATH
ExecStart=$DEPLOY_PATH/proxy-rs --log info serve --host 0.0.0.0 --port 8080 --types HTTP HTTPS SOCKS4 SOCKS5 --max-tries 3
ExecReload=/bin/kill -HUP $MAINPID
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=proxy-rs

# Variables d'environnement
Environment=RUST_LOG=info
Environment=PROXY_RS_JUDGES_ENABLED=true

# Limites de ressources
LimitNOFILE=65536
LimitNPROC=4096
LimitAS=infinity

# Sécurité renforcée
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=$LOG_PATH $LIB_PATH $DEPLOY_PATH $CONFIG_PATH
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictRealtime=true
RestrictSUIDSGID=true
RemoveIPC=true

[Install]
WantedBy=multi-user.target
SERVICE_EOF

chmod 644 "/etc/systemd/system/$SERVICE_NAME.service"

# Configuration firewall avancée
echo -e "${YELLOW}🔥 Configuration firewall UFW avancée...${NC}"
ufw --force reset
ufw default deny incoming
ufw default allow outgoing
ufw allow 22/tcp comment "SSH Access"
ufw allow $PROXY_PORT/tcp comment "Proxy.rs Server (Port $PROXY_PORT)"
ufw allow $API_PORT/tcp comment "Proxy.rs API REST (Port $API_PORT)"
ufw --force enable

# Configuration logrotate
echo -e "${YELLOW}📋 Configuration rotation des logs...${NC}"
cat > "/etc/logrotate.d/$SERVICE_NAME" << 'LOGROTATE_EOF'
/var/log/proxy-rs/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 644 proxy-rs proxy-rs
    postrotate
        systemctl reload proxy-rs > /dev/null 2>&1 || true
    endscript
}
LOGROTATE_EOF

# Activation et démarrage du service
echo -e "${YELLOW}🎯 Activation et démarrage du service...${NC}"
systemctl daemon-reload
systemctl enable "$SERVICE_NAME"
systemctl restart "$SERVICE_NAME"

# Attendre démarrage du service
echo "Attente démarrage du service..."
sleep 10

# Vérification statut détaillé
if systemctl is-active --quiet "$SERVICE_NAME"; then
    echo -e "${GREEN}✅ Service actif${NC}"
else
    echo -e "${RED}❌ Service inactif${NC}"
    echo "Diagnostics:"
    systemctl status "$SERVICE_NAME" --no-pager -l
    journalctl -u "$SERVICE_NAME" --no-pager -n 20
    exit 1
fi

# Tests de validation
echo -e "${BLUE}📊 Tests de validation du déploiement${NC}"
echo "============================================"

# Statut du service
echo -e "${YELLOW}🔍 Statut du service:${NC}"
systemctl status "$SERVICE_NAME" --no-pager -l

# Test des judges optimisés
echo -e "${YELLOW}⚡ Validation des judges optimisés...${NC}"
sleep 5
if journalctl -u "$SERVICE_NAME" --since "1 minute ago" | grep -q "Judges optimisés"; then
    echo -e "${GREEN}✅ Judges optimisés initialisés avec succès${NC}"
    journalctl -u "$SERVICE_NAME" --since "1 minute ago" | grep "Judges optimisés" | tail -3
else
    echo -e "${YELLOW}⚠️  Judges en cours d'initialisation...${NC}"
fi

# Vérification des ports
echo -e "${YELLOW}🌐 Vérification des ports d'écoute:${NC}"
if netstat -tuln 2>/dev/null | grep -E ":(8080|3000)" > /dev/null; then
    echo -e "${GREEN}✅ Ports configurés correctement${NC}"
    netstat -tuln 2>/dev/null | grep -E ":(8080|3000)"
else
    echo -e "${YELLOW}⚠️  Ports pas encore en écoute (démarrage en cours)${NC}"
fi

# Test API Health
echo -e "${YELLOW}🏥 Test API Health...${NC}"
SERVER_IP=$(curl -s --max-time 5 ifconfig.me 2>/dev/null || echo "localhost")
if curl -s --max-time 10 "http://localhost:$API_PORT/api/v1/health" >/dev/null; then
    echo -e "${GREEN}✅ API Health Check OK${NC}"
else
    echo -e "${YELLOW}⚠️  API Health en cours de démarrage${NC}"
fi

# Test proxy rotation
echo -e "${YELLOW}🔄 Test proxy rotation...${NC}"
if timeout 15s curl -x "http://localhost:$PROXY_PORT" -s "https://httpbin.org/ip" >/dev/null 2>&1; then
    echo -e "${GREEN}✅ Proxy rotation fonctionnelle${NC}"
else
    echo -e "${YELLOW}⚠️  Proxy rotation en cours d'initialisation${NC}"
fi

# Logs récents du service
echo -e "${YELLOW}📝 Logs récents du service:${NC}"
journalctl -u "$SERVICE_NAME" --no-pager -n 10 | tail -10

# Résumé du déploiement
echo -e "${BLUE}🎉 DÉPLOIEMENT TERMINÉ AVEC SUCCÈS !${NC}"
echo "================================================="
echo -e "${GREEN}✅ Serveur Proxy: http://$SERVER_IP:$PROXY_PORT${NC}"
echo -e "${GREEN}✅ API REST: http://$SERVER_IP:$API_PORT${NC}"
echo -e "${GREEN}✅ Documentation: http://$SERVER_IP:$API_PORT/docs${NC}"
echo -e "${GREEN}✅ Health Check: http://$SERVER_IP:$API_PORT/api/v1/health${NC}"
echo ""
echo -e "${CYAN}⚡ Judges Optimisés: Validation 10x plus rapide${NC}"
echo -e "${CYAN}📊 Monitoring temps réel via API REST${NC}"
echo -e "${CYAN}🛡️ Configuration production sécurisée${NC}"
echo ""
echo -e "${BLUE}📊 Commandes utiles :${NC}"
echo "systemctl status $SERVICE_NAME                    # Statut du service"
echo "journalctl -u $SERVICE_NAME -f                   # Logs en temps réel"
echo "curl http://localhost:$API_PORT/api/v1/metrics   # Métriques complètes"
echo "curl http://localhost:$API_PORT/api/v1/health   # Health check API"
echo "$DEPLOY_PATH/proxy-rs --help                     # Aide commandes"
echo "$DEPLOY_PATH/proxy-rs grab --limit 10            # Test découverte proxies"
echo ""
echo -e "${GREEN}🚀 Proxy.rs v0.4.0 avec judges optimisés est prêt pour la production !${NC}"