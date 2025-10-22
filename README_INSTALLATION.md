# 🚀 Guide d'Installation Rapide Proxy.rs v0.4.0

## Installation Automatisée (Recommandé)

### Option 1: Installation One-Liner ⚡
```bash
curl -sSL https://raw.githubusercontent.com/duan78/proxy-rs/main/install.sh | bash
```

### Option 2: Téléchargement Manuel
```bash
# Télécharger le script
curl -O https://raw.githubusercontent.com/duan78/proxy-rs/main/install.sh

# Rendre exécutable
chmod +x install.sh

# Exécuter l'installation
sudo ./install.sh
```

## Installation sur VPS Ubuntu/Debian

### Prérequis
- Ubuntu 20.04+ ou Debian 11+
- Accès root ou sudo
- 2GB RAM minimum, 1 CPU minimum

### Installation Complete
```bash
# Mettre à jour le système
sudo apt update && sudo apt upgrade -y

# Installer Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Cloner le projet
git clone https://github.com/duan78/proxy-rs.git /opt/proxy-rs
cd /opt/proxy-rs

# Compiler
cargo build --release

# Créer utilisateur service
sudo useradd -r -s /bin/false proxy-rs

# Créer service systemd
sudo tee /etc/systemd/system/proxy-rs.service > /dev/null <<EOF
[Unit]
Description=Proxy.rs High-Performance Proxy Server
After=network.target

[Service]
Type=simple
User=proxy-rs
WorkingDirectory=/opt/proxy-rs
ExecStart=/opt/proxy-rs/target/release/proxy-rs serve --host 0.0.0.0 --port 8080 --types HTTP HTTPS SOCKS4 SOCKS5
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

# Démarrer le service
sudo systemctl daemon-reload
sudo systemctl enable proxy-rs
sudo systemctl start proxy-rs

# Configurer firewall
sudo ufw allow 8080/tcp
sudo ufw allow 3000/tcp
```

## Vérification Installation

### Statut du Service
```bash
sudo systemctl status proxy-rs
sudo journalctl -u proxy-rs -f
```

### Tests de Fonctionnement
```bash
# Test API Health
curl http://localhost:3000/api/v1/health

# Test Proxy Rotation
curl -x http://localhost:8080 -s https://httpbin.org/ip

# Test Découverte Proxies
/opt/proxy-rs/target/release/proxy-rs grab --limit 10
```

## Configuration

### Fichier de Configuration
Le fichier de configuration se trouve dans `/etc/proxy-rs/proxy-rs.toml`

### Configuration Personnalisée
```bash
# Éditer la configuration
sudo nano /etc/proxy-rs/proxy-rs.toml

# Redémarrer le service
sudo systemctl restart proxy-rs
```

## Commandes Utiles

### Gestion du Service
```bash
sudo systemctl start proxy-rs      # Démarrer
sudo systemctl stop proxy-rs       # Arrêter
sudo systemctl restart proxy-rs    # Redémarrer
sudo systemctl status proxy-rs     # Statut
```

### Logs et Monitoring
```bash
sudo journalctl -u proxy-rs -f           # Logs en temps réel
sudo journalctl -u proxy-rs --since "1h" # Logs dernière heure
curl http://localhost:3000/api/v1/metrics # Métriques API
```

### Utilisation CLI
```bash
# Aide complète
/opt/proxy-rs/target/release/proxy-rs --help

# Découverte de proxies
/opt/proxy-rs/target/release/proxy-rs grab --limit 100

# Test de proxies
/opt/proxy-rs/target/release/proxy-rs find --types HTTP HTTPS --limit 50

# Lancer serveur manuellement
/opt/proxy-rs/target/release/proxy-rs serve --host 0.0.0.0 --port 8080 --types HTTP HTTPS
```

## Ports par Défaut

- **Serveur Proxy**: 8080
- **API REST**: 3000
- **Documentation**: http://votre-ip:3000/docs

## Dépannage

### Service ne démarre pas
```bash
# Vérifier les logs
sudo journalctl -u proxy-rs -n 20

# Vérifier le binaire
ls -la /opt/proxy-rs/target/release/proxy-rs

# Tester manuellement
sudo -u proxy-rs /opt/proxy-rs/target/release/proxy-rs serve --help
```

### Ports bloqués
```bash
# Vérifier firewall
sudo ufw status

# Vérifier ports ouverts
sudo netstat -tuln | grep -E ':(8080|3000)'
```

### Compilation échoue
```bash
# Nettoyer et recompiler
cd /opt/proxy-rs
cargo clean
cargo build --release

# Vérifier dépendances
sudo apt install build-essential pkg-config libssl-dev
```

## Mise à Jour

### Mise à jour automatique
```bash
cd /opt/proxy-rs
git pull origin main
cargo build --release
sudo systemctl restart proxy-rs
```

## Support

- **GitHub**: https://github.com/duan78/proxy-rs
- **Issues**: https://github.com/duan78/proxy-rs/issues
- **Documentation**: https://github.com/duan78/proxy-rs/blob/main/README.md

---

**Proxy.rs v0.4.0 - Installation terminée avec succès ! 🚀**