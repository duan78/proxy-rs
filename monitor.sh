#!/bin/bash

# Script de monitoring pour Proxy.rs en production
# À exécuter sur le serveur

SERVER="root@217.154.180.62"
SERVICE_NAME="proxy-rs"

echo "📊 Monitoring Proxy.rs - $(date)"

# Statut du service
echo "=== STATUT SERVICE ==="
ssh $SERVER "systemctl status $SERVICE_NAME --no-pager"

# Ressources utilisées
echo -e "\n=== RESSOURCES SYSTEME ==="
ssh $SERVER "ps aux | grep proxy-rs | grep -v grep"

# Connexions réseau
echo -e "\n=== CONNEXIONS RÉSEAU ==="
ssh $SERVER "netstat -an | grep :8080 | wc -l && echo 'connexions actives sur port 8080'"

# Logs récents
echo -e "\n=== DERNIERS LOGS ==="
ssh $SERVER "journalctl -u $SERVICE_NAME --since '10 minutes ago' --no-pager"

# Disque disponible
echo -e "\n=== ESPACE DISQUE ==="
ssh $SERVER "df -h /"

# Mémoire disponible
echo -e "\n=== MÉMOIRE ==="
ssh $SERVER "free -h"

echo -e "\n✅ Monitoring terminé"