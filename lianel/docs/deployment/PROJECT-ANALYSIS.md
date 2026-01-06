# Lianel Project - Comprehensive Analysis

**Date**: December 2024  
**Purpose**: Deep research analysis of requirements, roadmap, development, and deployment strategies

---

## Executive Summary

The **Lianel Infrastructure** is a containerized web application platform deployed on a single VPS server (72.60.80.84) that provides:

1. **Current Platform**: React frontend, Rust backend (Profile Service), Apache Airflow orchestration, Keycloak SSO, comprehensive monitoring
2. **Future Platform**: EU Energy & Geospatial Intelligence Platform (data pipeline system)
3. **Deployment Model**: **100% remote deployment** - no local execution, build images locally, transfer via SCP, deploy on remote host

---

## Part 1: Requirements & Roadmap

### Current System Requirements

#### 1.1 Infrastructure Requirements
- **Platform**: Single VPS (72.60.80.84)
- **OS**: Ubuntu/Debian Linux
- **Orchestration**: Docker Compose
- **Domains**: 
  - `lianel.se`, `www.lianel.se` (main site)
  - `auth.lianel.se` (Keycloak SSO)
  - `airflow.lianel.se` (Airflow UI)
  - `monitoring.lianel.se` (Grafana)

#### 1.2 Service Requirements
- **Frontend**: React 18 application with user profile management
- **Backend**: Rust-based Profile Service API (port 3000/9000)
- **Authentication**: Keycloak 26.4.6 + OAuth2 Proxy
- **Orchestration**: Apache Airflow 3.1.3 (CeleryExecutor)
- **Monitoring**: Prometheus + Grafana + Loki + Promtail
- **Reverse Proxy**: Nginx with SSL/TLS (Let's Encrypt)
- **Databases**: PostgreSQL (host-level, not containerized)

#### 1.3 Security Requirements
- SSO across all services via Keycloak
- OAuth2 Proxy as authentication gateway
- TLS 1.2/1.3 encryption
- Firewall (UFW) configuration
- Secrets management via environment variables

### Future Platform Requirements (EU Energy Platform)

#### 1.4 Data Platform Requirements
**Phase 0: Documentation Enhancement** (Current Phase - 2-3 weeks)
- Complete technical specifications
- Assess Eurostat + NUTS data coverage
- Finalize storage architecture decisions
- Create detailed Airflow DAG specifications

**Phase 1: Foundation Setup** (2-3 weeks)
- Database infrastructure (PostgreSQL + PostGIS)
- Airflow configuration for data pipelines
- Monitoring and alerting setup
- Development environment

**Phase 2: Eurostat + NUTS Implementation** (4-6 weeks)
- Eurostat data ingestion pipeline
- NUTS geospatial data processing
- Data harmonization and validation
- Initial ML dataset creation

**Phase 3: Data Coverage Assessment** (1-2 weeks)
- Evaluate data sufficiency
- Decision point: proceed to ML or add more sources

**Phase 4: ML & Analytics** (4-6 weeks) - if sufficient
- Create ML datasets (forecasting, clustering, geo-enrichment)
- Analytical dashboards
- Self-service data access

**Phase 5: Additional Sources** (6-8 weeks) - if needed
- ENTSO-E integration (high-frequency electricity data)
- OSM geospatial enrichment

**Phase 6: Operationalization** (3-4 weeks)
- Performance optimization
- Advanced monitoring
- Operational procedures

**Phase 7: Continuous Improvement** (Ongoing)

#### 1.5 Data Sources
- **Eurostat**: Energy statistics (annual data)
- **NUTS**: Geospatial boundaries (NUTS 0, 1, 2 levels)
- **ENTSO-E**: High-frequency electricity data (future)
- **OSM**: OpenStreetMap geospatial features (future)

#### 1.6 Technology Decisions
- **Database**: PostgreSQL + PostGIS (already in stack, can add TimescaleDB later)
- **Orchestration**: Apache Airflow (already deployed)
- **Storage**: Docker volumes + host filesystem
- **Monitoring**: Prometheus + Grafana (already deployed)

---

## Part 2: Development & Deployment Strategy

### 2.1 Core Principle: Remote-Only Deployment

**Key Constraint**: 
> "We never run on local, everything runs on remote host just by SSH connect. We just build the image and SCP to target"

This means:
- ✅ Build Docker images **locally** (on developer machine)
- ✅ Save images as tar/tar.gz files
- ✅ Transfer to remote host via **SCP**
- ✅ Load images on remote host via **SSH**
- ✅ Deploy containers on remote host via **SSH**
- ❌ **NO** local execution of services
- ❌ **NO** docker-compose up on local machine

### 2.2 Deployment Workflow

#### Standard Deployment Process

```bash
# ============================================
# STEP 1: Build Image Locally (with platform flag)
# ============================================
cd /path/to/hosting-base/lianel/dc

# Frontend
docker build --platform linux/amd64 \
  -t lianel-frontend:latest \
  -f frontend/Dockerfile frontend/

# OR Backend
docker build --platform linux/amd64 \
  -t lianel-profile-service:latest \
  -f profile-service/Dockerfile profile-service/

# ============================================
# STEP 2: Save Image as Tar File
# ============================================
docker save lianel-frontend:latest | gzip > lianel-frontend-amd64.tar.gz

# ============================================
# STEP 3: Transfer to Remote Host via SCP
# ============================================
scp lianel-frontend-amd64.tar.gz root@72.60.80.84:/root/lianel/dc/

# ============================================
# STEP 4: Load Image on Remote Host (via SSH)
# ============================================
ssh root@72.60.80.84
cd /root/lianel/dc
docker load < lianel-frontend-amd64.tar.gz

# ============================================
# STEP 5: Deploy Container (via SSH)
# ============================================
# Option A: Using docker-compose.yaml (image only, no build)
docker compose -f docker-compose.yaml up -d frontend

# Option B: Using docker-compose.frontend.yaml (with --no-build flag)
docker compose -f docker-compose.frontend.yaml up -d --no-build frontend

# Option C: Direct docker command
docker stop lianel-frontend 2>/dev/null || true
docker rm lianel-frontend 2>/dev/null || true
docker run -d --name lianel-frontend \
  --network lianel-network \
  lianel-frontend:latest
```

### 2.3 Automated Deployment Scripts

#### Script: `build-and-deploy.sh`
**Location**: `lianel/dc/scripts/build-and-deploy.sh`

**Purpose**: Automated build → save → scp → load → deploy workflow

**Usage**:
```bash
cd lianel/dc/scripts
./build-and-deploy.sh [frontend|backend|all]
```

**What it does**:
1. Builds image locally with `--platform linux/amd64`
2. Saves image as tar file
3. SCPs to remote host (`/tmp/`)
4. SSHs to remote and loads image
5. Deploys container using docker-compose
6. Cleans up temporary files

**Key Features**:
- Handles both frontend and backend
- Error handling at each step
- Automatic cleanup
- Configurable via environment variables:
  - `REMOTE_HOST` (default: 72.60.80.84)
  - `REMOTE_USER` (default: root)
  - `REPO_DIR` (default: /root/lianel/dc)

#### Script: `deploy-frontend.sh`
**Location**: `lianel/dc/deploy-frontend.sh`

**Purpose**: Frontend-specific deployment (runs on remote host)

**Usage**: Run on remote host after image is loaded

**What it does**:
1. Ensures `lianel-network` exists
2. Removes old container
3. Starts new container
4. Fixes network configuration
5. Verifies deployment

#### Script: `DEPLOY_COMMANDS.sh`
**Location**: Root directory

**Purpose**: Systematic deployment of all services

**Usage**: Run on remote host via SSH
```bash
ssh root@72.60.80.84 'bash -s' < DEPLOY_COMMANDS.sh
```

**What it does**:
1. Deploys infrastructure (postgres, keycloak, nginx)
2. Deploys OAuth2-Proxy
3. Deploys main services (frontend, profile-service)
4. Network verification
5. Health checks

### 2.4 Docker Compose File Strategy

#### File Organization

The project uses **multiple docker-compose files** for separation of concerns:

1. **`docker-compose.infra.yaml`**: Infrastructure services
   - PostgreSQL (containerized)
   - Keycloak
   - Nginx reverse proxy

2. **`docker-compose.oauth2-proxy.yaml`**: Authentication services
   - OAuth2 Proxy

3. **`docker-compose.frontend.yaml`**: Frontend service
   - **Contains `build:` section** (for local development)
   - **Contains `image:` section** (for remote deployment)
   - ⚠️ **Note**: On remote, use `--no-build` flag or use `docker-compose.yaml`

4. **`docker-compose.backend.yaml`**: Backend service
   - Profile Service
   - **Contains `build:` section** (for local development)

5. **`docker-compose.yaml`**: Main compose file
   - **Image-only definitions** (no build sections)
   - Used for remote deployment after images are loaded

6. **`docker-compose.airflow.yaml`**: Airflow services
   - Scheduler, Worker, Triggerer, DAG Processor, API Server
   - Redis for Celery broker

7. **`docker-compose.monitoring.yaml`**: Monitoring stack
   - Prometheus, Grafana, Loki, Promtail, cAdvisor, Node Exporter

#### Deployment Strategy for Compose Files

**On Remote Host**:
```bash
# Option 1: Use docker-compose.yaml (image only)
docker compose -f docker-compose.yaml up -d frontend

# Option 2: Use service-specific file with --no-build
docker compose -f docker-compose.frontend.yaml up -d --no-build frontend

# Option 3: Combine multiple files
docker compose -f docker-compose.infra.yaml -f docker-compose.backend.yaml up -d --no-build profile-service
```

### 2.5 Platform-Specific Builds

#### Critical: Cross-Platform Builds

**Problem**: Developer machines may be ARM (Mac M1/M2), remote host is AMD64

**Solution**: Always use `--platform linux/amd64` flag

```bash
# Frontend (Node.js)
docker build --platform linux/amd64 \
  -t lianel-frontend:latest \
  -f frontend/Dockerfile frontend/

# Backend (Rust)
docker build --platform linux/amd64 \
  -t lianel-profile-service:latest \
  -f profile-service/Dockerfile profile-service/
```

**Rust-Specific**: The Profile Service Dockerfile already handles cross-compilation:
- Uses `rust:1.83-alpine` with `x86_64-unknown-linux-musl` target
- Produces statically linked binary for AMD64

### 2.6 Image Transfer Optimization

#### Compression
```bash
# Save with compression (smaller file)
docker save lianel-frontend:latest | gzip > lianel-frontend-amd64.tar.gz

# Load compressed image
gunzip -c lianel-frontend-amd64.tar.gz | docker load
# OR
docker load < lianel-frontend-amd64.tar.gz  # Docker handles gzip automatically
```

#### File Sizes (Typical)
- Frontend image: ~133MB (65MB compressed)
- Backend image: ~50-100MB (varies)

### 2.7 Network Configuration

#### Docker Network: `lianel-network`
- **Type**: Bridge network
- **Purpose**: Isolated network for all services
- **Creation**: 
  ```bash
  docker network create lianel-network
  ```
- **All services** must be on this network for inter-service communication

#### Host Network Access
- PostgreSQL: Accessed via `host.docker.internal` or `172.17.0.1:5432`
- Docker daemon metrics: `127.0.0.1:9323` (for Prometheus)

### 2.8 Environment Variables

#### Location: `/root/lianel/dc/.env`

**Key Variables**:
```bash
# Airflow
AIRFLOW__CORE__FERNET_KEY=<generated>
_AIRFLOW_WWW_USER_PASSWORD=<strong password>
POSTGRES_PASSWORD=<strong password>

# Keycloak
KEYCLOAK_ADMIN_PASSWORD=<strong password>
KEYCLOAK_DB_PASSWORD=<strong password>

# OAuth2 Proxy
OAUTH2_CLIENT_SECRET=<from Keycloak>
OAUTH2_COOKIE_SECRET=<generated>

# Grafana
GRAFANA_ADMIN_PASSWORD=<strong password>
GRAFANA_OAUTH_CLIENT_SECRET=<from Keycloak>

# Redis
REDIS_PASSWORD=<strong password>
```

**Security**: `.env` file is **NOT** in git repository

---

## Part 3: Practical Deployment Examples

### 3.1 Frontend Deployment (Step-by-Step)

```bash
# ============================================
# LOCAL MACHINE
# ============================================

# 1. Navigate to project
cd /path/to/hosting-base/lianel/dc

# 2. Build for AMD64 platform
docker build --platform linux/amd64 \
  -t lianel-frontend:latest \
  -f frontend/Dockerfile frontend/

# 3. Save image
docker save lianel-frontend:latest | gzip > /tmp/lianel-frontend-amd64.tar.gz

# 4. Transfer to remote
scp /tmp/lianel-frontend-amd64.tar.gz root@72.60.80.84:/tmp/

# ============================================
# REMOTE HOST (via SSH)
# ============================================

# 5. SSH to remote
ssh root@72.60.80.84

# 6. Load image
cd /root/lianel/dc
docker load < /tmp/lianel-frontend-amd64.tar.gz

# 7. Stop and remove old container
docker stop lianel-frontend 2>/dev/null || true
docker rm lianel-frontend 2>/dev/null || true

# 8. Deploy new container
docker compose -f docker-compose.yaml up -d frontend

# 9. Verify
docker ps | grep lianel-frontend
curl -sk https://lianel.se/ | head -20

# 10. Cleanup
rm -f /tmp/lianel-frontend-amd64.tar.gz
```

### 3.2 Backend Deployment (Step-by-Step)

```bash
# ============================================
# LOCAL MACHINE
# ============================================

cd /path/to/hosting-base/lianel/dc

# Build Rust service for AMD64
docker build --platform linux/amd64 \
  -t lianel-profile-service:latest \
  -f profile-service/Dockerfile profile-service/

# Save and transfer
docker save lianel-profile-service:latest | gzip > /tmp/profile-service-amd64.tar.gz
scp /tmp/profile-service-amd64.tar.gz root@72.60.80.84:/tmp/

# ============================================
# REMOTE HOST
# ============================================

ssh root@72.60.80.84
cd /root/lianel/dc

# Load image
docker load < /tmp/profile-service-amd64.tar.gz

# Deploy (using multiple compose files for dependencies)
docker compose -f docker-compose.infra.yaml \
               -f docker-compose.backend.yaml \
               up -d --no-build profile-service

# Verify
docker ps | grep lianel-profile-service
curl -s http://lianel-profile-service:3000/health || echo "Service starting..."
```

### 3.3 Full System Deployment

```bash
# ============================================
# REMOTE HOST (via SSH)
# ============================================

ssh root@72.60.80.84

# Use the systematic deployment script
cd /root/lianel/dc
bash /root/DEPLOY_COMMANDS.sh

# OR manually:
# Step 1: Infrastructure
docker compose -f docker-compose.infra.yaml up -d
sleep 20

# Step 2: OAuth2 Proxy
docker compose -f docker-compose.oauth2-proxy.yaml up -d
sleep 5

# Step 3: Main Services (after images loaded)
docker compose -f docker-compose.yaml up -d
sleep 10

# Step 4: Airflow
docker compose -f docker-compose.airflow.yaml up -d

# Step 5: Monitoring
docker compose -f docker-compose.monitoring.yaml up -d

# Verify all services
docker ps --format 'table {{.Names}}\t{{.Status}}'
```

### 3.4 Using Automated Scripts

```bash
# ============================================
# LOCAL MACHINE
# ============================================

cd /path/to/hosting-base/lianel/dc/scripts

# Deploy frontend only
./build-and-deploy.sh frontend

# Deploy backend only
./build-and-deploy.sh backend

# Deploy both
./build-and-deploy.sh all

# Custom remote host
REMOTE_HOST=192.168.1.100 ./build-and-deploy.sh frontend
```

---

## Part 4: CI/CD Integration (GitHub Actions)

### 4.1 GitHub Actions Workflow

The project has GitHub Actions workflows for automated deployment:

**Location**: `.github/workflows/`

**Workflows**:
1. `deploy-frontend.yml`: Frontend deployment
2. `deploy-profile-service.yml`: Backend deployment

**Process**:
1. Build Docker image in GitHub Actions runner
2. Push to GitHub Container Registry (ghcr.io)
3. Save image as tar.gz
4. SCP to remote host
5. SSH to remote and load/deploy

**Key Features**:
- Uses SSH keys stored in GitHub Secrets
- Handles image tagging automatically
- Includes health checks
- Rollback capability

### 4.2 Required GitHub Secrets

```
REMOTE_HOST=72.60.80.84
REMOTE_USER=root
REMOTE_PORT=22
SSH_PRIVATE_KEY=<private key content>
```

---

## Part 5: Key Learnings & Best Practices

### 5.1 Critical Practices

1. **Always use `--platform linux/amd64`** when building locally
2. **Never run services locally** - only build images
3. **Always compress images** before transfer (gzip)
4. **Use `--no-build` flag** when deploying on remote
5. **Verify network** (`lianel-network`) exists before deployment
6. **Check image is loaded** before deploying container
7. **Use multiple compose files** for modular deployment

### 5.2 Common Pitfalls

1. **Forgetting platform flag**: Results in ARM images that won't run on AMD64
2. **Building on remote**: Don't use `build:` sections on remote host
3. **Network issues**: Services can't communicate if not on `lianel-network`
4. **Image not loaded**: Container fails if image doesn't exist locally on remote
5. **Environment variables**: `.env` file must exist on remote host

### 5.3 Troubleshooting

#### Issue: Container won't start
```bash
# Check if image exists
docker images | grep lianel-frontend

# Check network exists
docker network ls | grep lianel-network

# Check logs
docker logs lianel-frontend

# Check container status
docker ps -a | grep lianel-frontend
```

#### Issue: Services can't communicate
```bash
# Verify network membership
docker network inspect lianel-network

# Check container network
docker inspect lianel-frontend | grep NetworkMode
```

#### Issue: Build fails (wrong platform)
```bash
# Rebuild with platform flag
docker build --platform linux/amd64 -t lianel-frontend:latest -f frontend/Dockerfile frontend/
```

---

## Part 6: Architecture Overview

### 6.1 Service Architecture

```
Internet
  ↓ HTTPS
Nginx (Reverse Proxy)
  ↓
OAuth2 Proxy (Auth Gateway)
  ↓
Keycloak (SSO)
  ↓
Services:
  - Frontend (React)
  - Profile Service (Rust API)
  - Airflow (Data Orchestration)
  - Grafana (Monitoring)
```

### 6.2 Data Flow

1. **User Request** → Nginx (port 443)
2. **Auth Check** → OAuth2 Proxy
3. **SSO** → Keycloak
4. **Proxy** → Backend Service
5. **Response** → User

### 6.3 Monitoring Flow

1. **Metrics Collection**: cAdvisor, Node Exporter
2. **Scraping**: Prometheus (every 15s)
3. **Log Collection**: Promtail → Loki
4. **Visualization**: Grafana dashboards
5. **Access**: https://www.lianel.se/monitoring/

---

## Part 7: Future Platform Integration

### 7.1 Integration Points

The future EU Energy Platform will integrate with existing infrastructure:

1. **Airflow**: Already deployed, will host data pipeline DAGs
2. **PostgreSQL**: Will add PostGIS extension for geospatial data
3. **Grafana**: Will add dashboards for energy metrics
4. **Frontend**: Will add energy data visualization pages

### 7.2 Deployment Strategy for Data Platform

Same remote-only deployment model:
1. Build Airflow DAG containers locally
2. SCP to remote
3. Load and deploy
4. Monitor via existing Grafana setup

---

## Part 8: Documentation References

### Key Documents

1. **System Overview**: `lianel/docs/P0-01-SYSTEM-OVERVIEW.md`
2. **Deployment Guide**: `lianel/docs/P0-04-DEPLOYMENT-GUIDE.md`
3. **Remote Host Analysis**: `lianel/docs/P0-07-REMOTE-HOST-ANALYSIS.md`
4. **Roadmap**: `lianel/docs/08-implementation/01-roadmap.md`
5. **Quick Start**: `lianel/docs/08-implementation/02-phase-0-quick-start.md`
6. **Deployment Notes**: `lianel/dc/DEPLOYMENT-NOTES.md`
7. **Deployment Guide (DC)**: `lianel/dc/DEPLOYMENT-GUIDE.md`

### Scripts

1. **Build & Deploy**: `lianel/dc/scripts/build-and-deploy.sh`
2. **Deploy Frontend**: `lianel/dc/deploy-frontend.sh`
3. **System Deployment**: `DEPLOY_COMMANDS.sh`

---

## Summary

### Core Principles

1. ✅ **Remote-only deployment**: Build locally, deploy remotely
2. ✅ **SCP-based transfer**: Images transferred via SCP
3. ✅ **SSH-based execution**: All remote operations via SSH
4. ✅ **Platform-aware builds**: Always build for `linux/amd64`
5. ✅ **Modular compose files**: Separate files for different concerns
6. ✅ **Image-first deployment**: Load images, then deploy containers

### Workflow Summary

```
Local Machine                    Remote Host (72.60.80.84)
=============                   ========================
1. Build Image                  (via SSH)
   --platform linux/amd64       
                               2. Load Image
2. Save Image                     docker load < image.tar.gz
   docker save | gzip            
                               3. Deploy Container
3. Transfer via SCP               docker compose up -d
   scp image.tar.gz              
                               4. Verify
4. (Optional) Deploy via SSH      docker ps, curl, etc.
   ssh ... docker compose up
```

### Key Takeaways

- **Never run services locally** - only build images
- **Always specify platform** - `--platform linux/amd64`
- **Use SCP for transfer** - standard method
- **Load before deploy** - images must exist on remote
- **Use compose files strategically** - image-only for remote
- **Network is critical** - all services on `lianel-network`

---

**Last Updated**: December 2024  
**Status**: Comprehensive analysis complete


