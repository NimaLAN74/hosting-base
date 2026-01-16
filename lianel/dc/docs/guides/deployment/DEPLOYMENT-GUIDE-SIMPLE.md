# Deployment Guide - Quick Start
**Last Updated**: January 15, 2026

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Quick Deployment](#quick-deployment)
3. [Service Configuration](#service-configuration)
4. [Verification](#verification)
5. [Common Issues](#common-issues)

---

## Prerequisites

### Server Requirements
- **OS**: Ubuntu 20.04+ or Debian 11+
- **RAM**: 8 GB minimum (16 GB recommended)
- **CPU**: 2 cores minimum (4 cores recommended)
- **Disk**: 100 GB minimum
- **Network**: Public IP address

### Software Requirements
- Docker Engine 20.10+
- Docker Compose 2.0+
- Git

### Domain Setup
Configure DNS A records pointing to your server:
- `lianel.se` → Server IP
- `www.lianel.se` → Server IP
- `auth.lianel.se` → Server IP
- `airflow.lianel.se` → Server IP
- `monitoring.lianel.se` → Server IP

---

## Quick Deployment

### Step 1: Install Docker

```bash
# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# Install Docker Compose
sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
sudo chmod +x /usr/local/bin/docker-compose

# Verify
docker --version
docker-compose --version
```

### Step 2: Clone Repository

```bash
cd /root
git clone https://github.com/NimaLAN74/hosting-base.git
cd hosting-base/lianel/dc
```

### Step 3: Configure Environment

```bash
# Copy example env file
cp .env.example .env

# Edit .env file with your values
nano .env
```

**Required variables**:
- `KEYCLOAK_ADMIN_PASSWORD`: Strong password for Keycloak admin
- `KEYCLOAK_DB_PASSWORD`: Database password for Keycloak
- `POSTGRES_PASSWORD`: PostgreSQL password
- `AIRFLOW__CORE__FERNET_KEY`: Generate with: `python3 -c "from cryptography.fernet import Fernet; print(Fernet.generate_key().decode())"`

### Step 4: Setup PostgreSQL (Host-level)

```bash
# Install PostgreSQL
sudo apt install -y postgresql postgresql-contrib

# Create databases
sudo -u postgres psql << EOF
CREATE DATABASE airflow;
CREATE USER airflow WITH PASSWORD '<POSTGRES_PASSWORD>';
GRANT ALL PRIVILEGES ON DATABASE airflow TO airflow;

CREATE DATABASE keycloak;
CREATE USER keycloak WITH PASSWORD '<KEYCLOAK_DB_PASSWORD>';
GRANT ALL PRIVILEGES ON DATABASE keycloak TO keycloak;
EOF
```

### Step 5: Create Docker Network

```bash
docker network create lianel-network
```

### Step 6: Deploy Services

```bash
# Deploy infrastructure (Nginx, Keycloak)
docker-compose -f docker-compose.infra.yaml up -d

# Wait for Keycloak to start (30 seconds)
sleep 30

# Deploy application services
docker-compose -f docker-compose.yaml up -d

# Deploy Airflow
docker-compose -f docker-compose.airflow.yaml up -d

# Deploy monitoring
docker-compose -f docker-compose.monitoring.yaml up -d

# Deploy OAuth2 Proxy
docker-compose -f docker-compose.oauth2-proxy.yaml up -d
```

### Step 7: Setup SSL Certificates

```bash
# Install Certbot
sudo apt install -y certbot

# Obtain certificates
sudo certbot certonly --webroot -w /root/lianel/dc/nginx/html \
  -d lianel.se -d www.lianel.se

sudo certbot certonly --webroot -w /root/lianel/dc/nginx/html \
  -d auth.lianel.se

sudo certbot certonly --webroot -w /root/lianel/dc/nginx/html \
  -d airflow.lianel.se

sudo certbot certonly --webroot -w /root/lianel/dc/nginx/html \
  -d monitoring.lianel.se

# Setup auto-renewal
sudo systemctl enable certbot.timer
sudo systemctl start certbot.timer
```

### Step 8: Configure Keycloak

```bash
# Run Keycloak setup script
chmod +x setup-keycloak.sh
./setup-keycloak.sh

# Update .env with generated secrets
# OAUTH2_CLIENT_SECRET=<from script output>
# GRAFANA_OAUTH_CLIENT_SECRET=<from script output>

# Restart OAuth2 Proxy
docker-compose -f docker-compose.oauth2-proxy.yaml restart oauth2-proxy
```

---

## Service Configuration

### Keycloak Users

1. Access Keycloak Admin Console: `https://auth.lianel.se`
2. Login with admin credentials
3. Navigate to: Realm → Users → Add User
4. Create users and set passwords

### Airflow Variables

1. Access Airflow: `https://airflow.lianel.se`
2. Navigate to: Admin → Variables
3. Add required variables:
   - `ENTSOE_API_TOKEN`: Your ENTSO-E API token (if using)

### Grafana Dashboards

1. Access Grafana: `https://monitoring.lianel.se`
2. Login with SSO
3. Dashboards are auto-provisioned from `/monitoring/grafana/provisioning/dashboards/`

---

## Verification

### Check Service Status

```bash
# Check all containers
docker ps

# Check specific service
docker logs keycloak
docker logs nginx-proxy
docker logs airflow-webserver
```

### Test Endpoints

```bash
# Health check
curl https://www.lianel.se/api/v1/health

# Frontend
curl -I https://www.lianel.se

# Keycloak
curl -I https://auth.lianel.se
```

### Verify SSL

```bash
# Check SSL certificate
openssl s_client -connect www.lianel.se:443 -servername www.lianel.se
```

---

## Common Issues

### Issue: Services Not Starting

**Check logs**:
```bash
docker logs <container-name>
```

**Common causes**:
- Missing environment variables
- Database connection issues
- Port conflicts

### Issue: SSL Certificate Errors

**Verify**:
- DNS records are correct
- Certificates are in `/etc/letsencrypt/`
- Nginx can read certificates

**Fix**:
```bash
# Renew certificates
sudo certbot renew

# Restart Nginx
docker restart nginx-proxy
```

### Issue: Keycloak Not Accessible

**Check**:
- Keycloak container is running
- Database connection is working
- Port 8080 is exposed

**Fix**:
```bash
# Check Keycloak logs
docker logs keycloak

# Restart Keycloak
docker restart keycloak
```

### Issue: OAuth2 Proxy Errors

**Check**:
- Client secret is correct
- Redirect URIs are configured
- Keycloak is accessible

**Fix**:
```bash
# Verify OAuth2 Proxy configuration
docker logs oauth2-proxy

# Restart OAuth2 Proxy
docker restart oauth2-proxy
```

---

## Maintenance

### Update Services

```bash
# Pull latest code
cd /root/hosting-base
git pull

# Rebuild and restart services
cd lianel/dc
docker-compose -f docker-compose.yaml build
docker-compose -f docker-compose.yaml up -d
```

### Backup

```bash
# Backup databases
./scripts/backup-database.sh

# Backup configuration
tar -czf backup-$(date +%Y%m%d).tar.gz .env docker-compose*.yaml
```

### Monitoring

- **Grafana**: `https://monitoring.lianel.se`
- **Airflow**: `https://airflow.lianel.se`
- **System Health**: Check Grafana dashboards

---

## Next Steps

1. **Create Users**: Add users in Keycloak
2. **Configure DAGs**: Set up Airflow variables
3. **Review Dashboards**: Check Grafana dashboards
4. **Test API**: Verify API endpoints
5. **Set Up Alerts**: Configure monitoring alerts

---

**Status**: Active  
**Last Review**: January 15, 2026
