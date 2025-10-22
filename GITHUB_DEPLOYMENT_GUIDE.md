# 🚀 Guide Déploiement : GitHub → VPS

## Étape 1 : Créer Repository GitHub

### 1.1 Créer le repository sur GitHub
1. Connectez-vous à [GitHub](https://github.com)
2. Cliquez sur **"+"** en haut à droite → **"New repository"**
3. Configurez le repository :
   ```
   Repository name: proxy-rs
   Description: High-performance proxy server with REST API and hot-reload configuration
   Visibility: Public (ou Private si vous préférez)
   ⚠️ NE PAS cocher "Add a README file"
   ⚠️ NE PAS cocher "Add .gitignore"
   ⚠️ NE PAS cocher "Choose a license"
   ```
4. Cliquez sur **"Create repository"**

### 1.2 Obtenir l'URL du repository
GitHub vous montrera l'URL de votre repository :
```
https://github.com/VOTRE_USERNAME/proxy-rs.git
```

## Étape 2 : Envoyer le code sur GitHub

### 2.1 Connecter le repository local
```bash
# Naviguez vers votre projet
cd "C:\Users\duan7\Downloads\proxy.rs-main\proxy.rs-main"

# Ajoutez le remote origin (remplacez VOTRE_USERNAME)
git remote add origin https://github.com/VOTRE_USERNAME/proxy-rs.git

# Vérifiez la connexion
git remote -v
```

### 2.2 Envoyer le code
```bash
# Push vers GitHub (main branch)
git push -u origin main
```

### 2.3 Si vous avez des erreurs d'authentification
Configurez GitHub CLI ou utilisez Personal Access Token :
```bash
# Option 1: GitHub CLI
gh auth login

# Option 2: Personal Access Token
git remote set-url origin https://VOTRE_USERNAME:YOUR_TOKEN@github.com/VOTRE_USERNAME/proxy-rs.git
```

## Étape 3 : Configuration de la VPS

### 3.1 Choisir un fournisseur VPS
**Recommandés pour Proxy.rs :**
- **DigitalOcean** : $5-10/mois, excellent performance
- **Vultr** : $2.50/mois, très rapide
- **Linode** : $5/mois, support excellent
- **AWS EC2** : $3.50/mois (t2.micro), très fiable

### 3.2 Configuration VPS minimale requise
```
CPU: 2+ cores (recommandé 2-4 cores)
RAM: 2GB minimum (4GB+ recommandé)
Stockage: 20GB SSD
OS: Ubuntu 20.04+ ou 22.04+
Bande passante: 1TB+ (pour scraping intensif)
```

### 3.3 Installation dépendances sur VPS
```bash
# Mettre à jour le système
sudo apt update && sudo apt upgrade -y

# Installer Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Installer dépendances de compilation
sudo apt install -y build-essential pkg-config libssl-dev git

# Installer Systemd (généralement déjà installé)
sudo apt install -y systemd

# Vérifier installation
rustc --version
cargo --version
```

## Étape 4 : Déploiement sur VPS

### 4.1 Cloner le repository sur VPS
```bash
# Se connecter à votre VPS via SSH
ssh root@VOTRE_IP_VPS

# Cloner le repository
git clone https://github.com/VOTRE_USERNAME/proxy-rs.git
cd proxy-rs

# Vérifier les fichiers
ls -la
```

### 4.2 Compiler le projet
```bash
# Compiler en mode release (optimisé)
cargo build --release

# Vérifier que le binaire est créé
ls -la target/release/proxy-rs

# Tester l'exécution
./target/release/proxy-rs --help
```

### 4.3 Créer l'utilisateur service (sécurité)
```bash
# Créer utilisateur dédié (sécurité recommandée)
sudo useradd -r -s /bin/false proxy-rs
sudo usermod -L proxy-rs

# Créer les répertoires nécessaires
sudo mkdir -p /opt/proxy-rs
sudo mkdir -p /etc/proxy-rs
sudo mkdir -p /var/log/proxy-rs
sudo mkdir -p /var/lib/proxy-rs

# Copier le binaire
sudo cp target/release/proxy-rs /opt/proxy-rs/
sudo chmod +x /opt/proxy-rs/proxy-rs

# Copier la configuration
sudo cp proxy-rs.toml /etc/proxy-rs/
sudo cp deploy.sh /opt/proxy-rs/
sudo cp monitor.sh /opt/proxy-rs/

# Configurer les permissions
sudo chown -R proxy-rs:proxy-rs /opt/proxy-rs
sudo chown -R proxy-rs:proxy-rs /etc/proxy-rs
sudo chown -R proxy-rs:proxy-rs /var/log/proxy-rs
sudo chown -R proxy-rs:proxy-rs /var/lib/proxy-rs
```

### 4.4 Configuration production
```bash
# Éditer la configuration pour production
sudo nano /etc/proxy-rs/proxy-rs.toml

# Configuration production recommandée :
[general]
max_connections = 5000
default_timeout = 8
log_level = "warn"  # Moins de logs en production

[server]
host = "0.0.0.0"  # Écoute sur toutes les interfaces
port = 8080
max_clients = 2000

[api]
enabled = true
port = 3000
host = "0.0.0.0"
enable_cors = true

[dnsbl]
enabled = true
timeout_secs = 5
malicious_threshold = 2
```

### 4.5 Créer le service Systemd
```bash
sudo nano /etc/systemd/system/proxy-rs.service
```

Contenu du fichier service :
```ini
[Unit]
Description=Proxy.rs High-Performance Proxy Server
After=network.target

[Service]
Type=simple
User=proxy-rs
Group=proxy-rs
WorkingDirectory=/opt/proxy-rs
ExecStart=/opt/proxy-rs/proxy-rs serve --config /etc/proxy-rs/proxy-rs.toml
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

# Sécurité renforcée
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/log/proxy-rs /var/lib/proxy-rs /opt/proxy-rs

[Install]
WantedBy=multi-user.target
```

### 4.6 Activer et démarrer le service
```bash
# Recharger Systemd
sudo systemctl daemon-reload

# Activer le service au démarrage
sudo systemctl enable proxy-rs

# Démarrer le service
sudo systemctl start proxy-rs

# Vérifier le statut
sudo systemctl status proxy-rs

# Voir les logs en temps réel
sudo journalctl -u proxy-rs -f
```

## Étape 5 : Tests et Validation

### 5.1 Tests de base
```bash
# Test API Health
curl http://VOTRE_IP:3000/api/v1/health

# Test métriques
curl http://VOTRE_IP:3000/api/v1/metrics

# Test documentation
curl http://VOTRE_IP:3000/docs
```

### 5.2 Test proxy rotation
```bash
# Test avec un des proxies du projet
curl -x http://VOTRE_IP:8080 -s https://httpbin.org/ip

# Test rotation multiple
for i in {1..5}; do
  curl -x http://VOTRE_IP:8080 -s https://httpbin.org/ip
  sleep 1
done
```

### 5.3 Test découverte de proxies
```bash
# Test discovery (grab)
/opt/proxy-rs/proxy-rs grab --limit 10

# Test validation (find)
/opt/proxy-rs/proxy-rs find --types HTTP --levels High --limit 5
```

## Étape 6 : Monitoring et Maintenance

### 6.1 Scripts de monitoring (fournis dans le projet)
```bash
# Exécuter le script de monitoring
sudo /opt/proxy-rs/monitor.sh

# Ou monitoring manuel
sudo systemctl status proxy-rs
sudo journalctl -u proxy-rs --since "1 hour ago" | grep ERROR
```

### 6.2 Configuration firewall
```bash
# Autoriser les ports nécessaires
sudo ufw allow 8080/tcp comment "Proxy.rs Proxy Server"
sudo ufw allow 3000/tcp comment "Proxy.rs API REST"

# Activer le firewall
sudo ufw enable

# Vérifier les règles
sudo ufw status verbose
```

### 6.5 Monitoring avancé (optionnel)
```bash
# Installer monitoring avancé
sudo apt install -y htop iotop nethogs

# Monitoring ressources
htop                     # CPU/Mémoire
sudo iotop                 # I/O disque
sudo nethogs               # Réseau par processus
```

## Étape 7 : Mises à jour

### 7.1 Mettre à jour le code
```bash
# Sur votre machine locale (ou VPS)
git add .
git commit -m "Fix: Correction performance et sécurité"
git push origin main

# Sur la VPS
cd /opt/proxy-rs
git pull origin main
cargo build --release
sudo systemctl restart proxy-rs
```

### 7.2 Mettre à jour Rust
```bash
# Mettre à jour Rust toolchain
rustup update

# Rebuild le projet
cd /opt/proxy-rs
cargo build --release
sudo systemctl restart proxy-rs
```

## 🎯 Résultats Attendus

Après déploiement, vous devriez avoir :

- **✅ Serveur proxy** : `http://VOTRE_IP:8080`
- **✅ API REST** : `http://VOTRE_IP:3000`
- **✅ Documentation** : `http://VOTRE_IP:3000/docs`
- **✅ Service Systemd** : Démarrage automatique
- **✅ Monitoring** : Logs système et métriques

## 🚨 Dépannage Commun

### Si le service ne démarre pas :
```bash
# Vérifier les logs
sudo journalctl -u proxy-rs --no-pager -n 50

# Vérifier le binaire
sudo -u proxy-rs /opt/proxy-rs/proxy-rs --help

# Vérifier la configuration
sudo -u proxy-rs /opt/proxy-rs/proxy-rs serve --config /etc/proxy-rs/proxy-rs.toml
```

### Si les ports sont bloqués :
```bash
# Vérifier firewall
sudo ufw status

# Vérifier ports ouverts
sudo netstat -tuln | grep -E ':(8080|3000)'

# Vérifier processus
ps aux | grep proxy-rs
```

### Si le code ne compile pas :
```bash
# Vérifier Rust version
rustc --version

# Nettoyer et recompiler
cargo clean
cargo build --release

# Vérifier dépendances
cargo check
```

Votre Proxy.rs est maintenant déployé et fonctionnel ! 🚀