# Deployment Quick Reference Guide

**Remote Host**: 72.60.80.84  
**Remote User**: root  
**Remote Directory**: /root/lianel/dc  
**Network**: lianel-network

---

## 🚀 Quick Deployment Commands

### Frontend Deployment

```bash
# LOCAL: Build
cd lianel/dc
docker build --platform linux/amd64 -t lianel-frontend:latest -f frontend/Dockerfile frontend/

# LOCAL: Save & Transfer
docker save lianel-frontend:latest | gzip > /tmp/frontend.tar.gz
scp /tmp/frontend.tar.gz root@72.60.80.84:/tmp/

# REMOTE: Load & Deploy
ssh root@72.60.80.84
cd /root/lianel/dc
docker load < /tmp/frontend.tar.gz
docker compose -f docker-compose.yaml up -d frontend
rm /tmp/frontend.tar.gz
```

### Backend Deployment

```bash
# LOCAL: Build
cd lianel/dc
docker build --platform linux/amd64 -t lianel-profile-service:latest -f profile-service/Dockerfile profile-service/

# LOCAL: Save & Transfer
docker save lianel-profile-service:latest | gzip > /tmp/backend.tar.gz
scp /tmp/backend.tar.gz root@72.60.80.84:/tmp/

# REMOTE: Load & Deploy
ssh root@72.60.80.84
cd /root/lianel/dc
docker load < /tmp/backend.tar.gz
docker compose -f docker-compose.infra.yaml -f docker-compose.backend.yaml up -d --no-build profile-service
rm /tmp/backend.tar.gz
```

### Using Automated Script

```bash
cd lianel/dc/scripts
./build-and-deploy.sh frontend   # or backend, or all
```

---

## 🔍 Verification Commands

### Check Services
```bash
# On remote host
docker ps --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'
docker network inspect lianel-network
```

### Check Logs
```bash
docker logs lianel-frontend --tail 50
docker logs lianel-profile-service --tail 50
docker logs nginx-proxy --tail 50
```

### Health Checks
```bash
curl -sk https://lianel.se/ | head -20
curl -s http://lianel-profile-service:3000/health
curl -sk https://auth.lianel.se/realms/lianel/.well-known/openid-configuration
```

---

## 🛠️ Common Operations

### Restart Service
```bash
ssh root@72.60.80.84
cd /root/lianel/dc
docker compose -f docker-compose.yaml restart frontend
```

### Stop Service
```bash
ssh root@72.60.80.84
docker stop lianel-frontend
docker rm lianel-frontend
```

### View All Containers
```bash
ssh root@72.60.80.84
docker ps -a
```

### Clean Up Old Images
```bash
ssh root@72.60.80.84
docker image prune -a
```

### Reload Nginx Config
```bash
ssh root@72.60.80.84
docker exec nginx-proxy nginx -t
docker exec nginx-proxy nginx -s reload
```

---

## 📋 Full System Deployment

```bash
# On remote host
ssh root@72.60.80.84
cd /root/lianel/dc
bash /root/DEPLOY_COMMANDS.sh
```

Or manually:
```bash
# 1. Infrastructure
docker compose -f docker-compose.infra.yaml up -d

# 2. OAuth2 Proxy
docker compose -f docker-compose.oauth2-proxy.yaml up -d

# 3. Main Services (after images loaded)
docker compose -f docker-compose.yaml up -d

# 4. Airflow
docker compose -f docker-compose.airflow.yaml up -d

# 5. Monitoring
docker compose -f docker-compose.monitoring.yaml up -d
```

---

## ⚠️ Important Notes

1. **Always use `--platform linux/amd64`** when building
2. **Never use `build:` sections on remote** - use `--no-build` flag
3. **Ensure `lianel-network` exists**: `docker network create lianel-network`
4. **Load image before deploying**: `docker load < image.tar.gz`
5. **Compress images**: Use `gzip` to reduce transfer size

---

## 🔗 Key Files

- **Build Script**: `lianel/dc/scripts/build-and-deploy.sh`
- **Deploy Script**: `lianel/dc/deploy-frontend.sh`
- **System Deploy**: `DEPLOY_COMMANDS.sh`
- **Compose Files**: `lianel/dc/docker-compose*.yaml`
- **Environment**: `/root/lianel/dc/.env` (on remote)

---

## 📚 Full Documentation

See `PROJECT-ANALYSIS.md` for comprehensive analysis.


