#!/bin/bash

# Script de déploiement automatique pour Proxy.rs
# À exécuter sur votre machine locale

set -e

SERVER="root@217.154.180.62"
PROJECT_NAME="proxy-rs"
DEPLOY_PATH="/opt/$PROJECT_NAME"
SERVICE_NAME="proxy-rs"

echo "🚀 Déploiement de Proxy.rs en production..."

# Vérification de la connexion SSH
echo "📡 Test de connexion SSH..."
ssh -o ConnectTimeout=10 $SERVER "echo 'Connexion OK'" || {
    echo "❌ Erreur de connexion SSH au serveur $SERVER"
    exit 1
}

# Création du répertoire de déploiement
echo "📁 Création du répertoire $DEPLOY_PATH..."
ssh $SERVER "mkdir -p $DEPLOY_PATH"

# Synchronisation des fichiers (exclusion des éléments non nécessaires)
echo "📦 Synchronisation des fichiers..."
rsync -avz --progress \
    --exclude 'target/' \
    --exclude '.git/' \
    --exclude '*.md' \
    --exclude 'deploy.sh' \
    --exclude '.gitignore' \
    --exclude 'Cargo.lock' \
    ./ $SERVER:$DEPLOY_PATH/

# Installation de Rust si nécessaire
echo "🦀 Vérification de Rust..."
ssh $SERVER "command -v cargo >/dev/null 2>&1 || {
    echo 'Installation de Rust...'
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
}"

# Compilation en mode release
echo "🔨 Compilation en mode release..."
ssh $SERVER "cd $DEPLOY_PATH && source ~/.cargo/env && cargo build --release"

# Création du service systemd
echo "⚙️ Configuration du service systemd..."
ssh $SERVER "cat > /etc/systemd/system/$SERVICE_NAME.service << 'EOF'
[Unit]
Description=Proxy.rs Production Service
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=$DEPLOY_PATH
ExecStart=$DEPLOY_PATH/target/release/proxy-rs serve --host 0.0.0.0 --port 8080 --max-conn 1000
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF"

# Rechargement et démarrage du service
echo "🎯 Démarrage du service..."
ssh $SERVER "systemctl daemon-reload"
ssh $SERVER "systemctl enable $SERVICE_NAME"
ssh $SERVER "systemctl restart $SERVICE_NAME"

# Vérification du statut
echo "📊 Vérification du statut du service..."
ssh $SERVER "systemctl status $SERVICE_NAME --no-pager"

# Configuration firewall si ufw est disponible
echo "🔥 Configuration du firewall..."
ssh $SERVER "command -v ufw >/dev/null 2>&1 && {
    ufw allow 8080/tcp
    echo 'Firewall configuré pour le port 8080'
} || echo 'UFW non détecté, configuration firewall manuelle requise'"

echo "✅ Déploiement terminé !"
echo "🌐 Service disponible sur: http://217.154.180.62:8080"
echo "📝 Logs: journalctl -u $SERVICE_NAME -f"