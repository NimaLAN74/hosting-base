# Comp AI Deployment Plan
**Version**: 1.0  
**Date**: January 23, 2026  
**Status**: 📋 **PLANNING**

---

## 1. Overview

This document outlines the detailed plan for deploying Comp AI service on the remote host (72.60.80.84), following the existing infrastructure patterns and best practices.

### 1.1 Objectives
- Deploy Comp AI service as a containerized application
- Integrate with existing authentication (Keycloak)
- Expose service through Nginx reverse proxy
- Add monitoring and logging capabilities
- Set up CI/CD pipeline for automated deployments
- Ensure high availability and scalability

### 1.2 Assumptions
- Comp AI is a new service (not currently in codebase)
- Service can be containerized (Docker)
- Service requires authentication/authorization
- Service needs database access (if applicable)
- Service exposes HTTP/HTTPS endpoints

---

## 2. Prerequisites & Requirements

### 2.1 Infrastructure Requirements
- ✅ Remote host: `72.60.80.84` (already available)
- ✅ Docker & Docker Compose (already installed)
- ✅ Nginx reverse proxy (already configured)
- ✅ Keycloak for authentication (already running)
- ✅ PostgreSQL database (already available)
- ✅ Monitoring stack (Grafana, Prometheus, Loki - already running)
- ✅ SSL certificates (Let's Encrypt - already configured)

### 2.2 Service Requirements
- [ ] Comp AI service code/application
- [ ] Dockerfile for containerization
- [ ] Environment variables configuration
- [ ] API documentation (OpenAPI/Swagger preferred)
- [ ] Health check endpoint
- [ ] Database schema (if database required)

### 2.3 Network Requirements
- Internal port: `300X` (to be determined, e.g., 3002)
- External domain: `comp-ai.lianel.se` or `ai.lianel.se` (to be confirmed)
- Network: `lianel-network` (existing Docker network)

---

## 3. Architecture & Design

### 3.1 Service Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Internet Users                           │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│              Nginx Reverse Proxy (443/80)                   │
│  - SSL Termination                                          │
│  - OAuth2 Proxy (if needed)                                 │
│  - Rate Limiting                                            │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│              Comp AI Service (Port 300X)                     │
│  - Container: lianel-comp-ai-service                       │
│  - Framework: [To be determined]                           │
│  - Health: /health                                          │
└──────────────────────┬──────────────────────────────────────┘
                       │
        ┌──────────────┼──────────────┐
        │              │              │
        ▼              ▼              ▼
┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│  Keycloak   │ │ PostgreSQL  │ │  External   │
│  (Auth)     │ │  (Database) │ │  APIs       │
└─────────────┘ └─────────────┘ └─────────────┘
```

### 3.2 Integration Points
- **Authentication**: Keycloak OAuth2/OIDC
- **Database**: PostgreSQL (lianel_energy database or new database)
- **Monitoring**: Prometheus metrics, Loki logs
- **Service Discovery**: Docker network (lianel-network)

---

## 4. Deployment Steps

### 4.1 Phase 1: Service Preparation

#### 4.1.1 Create Service Directory Structure
```bash
lianel/dc/comp-ai-service/
├── src/                    # Service source code
├── Dockerfile              # Container definition
├── Cargo.toml             # (if Rust) or package.json (if Node.js)
├── .dockerignore
├── README.md
└── config/                 # Configuration files
```

#### 4.1.2 Service Implementation Checklist
- [ ] Implement service with health endpoint (`/health`)
- [ ] Add Keycloak authentication middleware
- [ ] Implement API endpoints
- [ ] Add database connection (if needed)
- [ ] Add Prometheus metrics endpoint (`/metrics`)
- [ ] Add structured logging
- [ ] Create OpenAPI/Swagger documentation
- [ ] Add error handling and validation
- [ ] Write unit tests
- [ ] Write integration tests

#### 4.1.3 Dockerfile Creation
```dockerfile
# Example Dockerfile (adjust based on service language)
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/comp-ai-service /usr/local/bin/
EXPOSE 3002
CMD ["comp-ai-service"]
```

### 4.2 Phase 2: Docker Compose Configuration

#### 4.2.1 Create `docker-compose.comp-ai.yaml`
```yaml
services:
  comp-ai-service:
    build:
      context: ./comp-ai-service
      dockerfile: Dockerfile
    image: lianel-comp-ai-service:latest
    container_name: lianel-comp-ai-service
    env_file:
      - .env
    environment:
      PORT: 3002
      POSTGRES_HOST: ${POSTGRES_HOST:-172.18.0.1}
      POSTGRES_PORT: ${POSTGRES_PORT:-5432}
      POSTGRES_USER: ${POSTGRES_USER:-postgres}
      POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
      POSTGRES_DB: ${POSTGRES_DB:-lianel_energy}
      KEYCLOAK_URL: https://auth.lianel.se
      KEYCLOAK_REALM: ${KEYCLOAK_REALM:-lianel}
      KEYCLOAK_ADMIN_USER: ${KEYCLOAK_ADMIN_USER}
      KEYCLOAK_ADMIN_PASSWORD: ${KEYCLOAK_ADMIN_PASSWORD}
      RUST_LOG: info
      # Add Comp AI specific environment variables
      COMP_AI_MODEL_PATH: ${COMP_AI_MODEL_PATH:-/models}
      COMP_AI_API_KEY: ${COMP_AI_API_KEY}
    expose:
      - "3002"
    depends_on:
      - keycloak
    restart: unless-stopped
    networks:
      - lianel-network
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3002/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s

networks:
  lianel-network:
    external: true
```

### 4.3 Phase 3: Nginx Configuration

#### 4.3.1 Add Location Block to `nginx/config/nginx.conf`
```nginx
# Comp AI Service at /api/v1/comp-ai/ (or /comp-ai/)
location /api/v1/comp-ai/ {
    # OAuth2 authentication
    auth_request /oauth2/auth;
    error_page 401 = /oauth2/sign_in;

    limit_req zone=general burst=20 nodelay;
    
    set $comp_ai_upstream "http://lianel-comp-ai-service:3002";
    proxy_pass $comp_ai_upstream;
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection 'upgrade';
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_cache_bypass $http_upgrade;
    
    # Timeouts
    proxy_connect_timeout 60s;
    proxy_send_timeout 60s;
    proxy_read_timeout 60s;
}
```

#### 4.3.2 Alternative: Dedicated Subdomain
If using a dedicated subdomain (e.g., `comp-ai.lianel.se`):
```nginx
server {
    listen 443 ssl;
    server_name comp-ai.lianel.se;

    ssl_certificate /etc/letsencrypt/live/comp-ai.lianel.se/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/comp-ai.lianel.se/privkey.pem;

    # OAuth2 authentication
    auth_request /oauth2/auth;
    error_page 401 = /oauth2/sign_in;

    location / {
        set $comp_ai_upstream "http://lianel-comp-ai-service:3002";
        proxy_pass $comp_ai_upstream;
        # ... (same proxy settings as above)
    }
}
```

### 4.4 Phase 4: Keycloak Integration

#### 4.4.1 Create Keycloak Client
```bash
# Script: scripts/keycloak-setup/create-comp-ai-keycloak-client.sh
# Creates a Keycloak client for Comp AI service
# - Client ID: comp-ai-service
# - Client Secret: (generated)
# - Valid Redirect URIs: https://www.lianel.se/api/v1/comp-ai/*
# - Service Accounts Enabled: Yes (for backend-to-backend auth)
```

#### 4.4.2 Environment Variables
Add to `.env`:
```bash
COMP_AI_KEYCLOAK_CLIENT_ID=comp-ai-service
COMP_AI_KEYCLOAK_CLIENT_SECRET=<generated-secret>
```

### 4.5 Phase 5: Database Setup (if needed)

#### 4.5.1 Create Database Schema
```sql
-- File: database/migrations/XXX_create_comp_ai_schema.sql
CREATE SCHEMA IF NOT EXISTS comp_ai;

CREATE TABLE IF NOT EXISTS comp_ai.requests (
    id BIGSERIAL PRIMARY KEY,
    user_id VARCHAR(255) NOT NULL,
    request_text TEXT NOT NULL,
    response_text TEXT,
    model_used VARCHAR(100),
    tokens_used INTEGER,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_comp_ai_requests_user_id ON comp_ai.requests(user_id);
CREATE INDEX idx_comp_ai_requests_created_at ON comp_ai.requests(created_at);
```

#### 4.5.2 Run Migration
```bash
docker exec dc-postgres-1 psql -U postgres -d lianel_energy -f /path/to/migration.sql
```

### 4.6 Phase 6: Monitoring & Logging

#### 4.6.1 Prometheus Metrics
- Add `/metrics` endpoint to service
- Configure Prometheus to scrape: `lianel-comp-ai-service:3002/metrics`
- Add to `monitoring/prometheus/prometheus.yml`:
```yaml
scrape_configs:
  - job_name: 'comp-ai-service'
    static_configs:
      - targets: ['lianel-comp-ai-service:3002']
```

#### 4.6.2 Grafana Dashboard
- Create dashboard: `monitoring/grafana/provisioning/dashboards/comp-ai-service.json`
- Metrics to monitor:
  - Request rate
  - Response time (p50, p95, p99)
  - Error rate
  - Token usage
  - Model inference time
  - Database query time

#### 4.6.3 Logging
- Ensure service logs to stdout/stderr
- Promtail will automatically collect logs from Docker container
- Add log labels in `monitoring/promtail/config.yml` if needed

### 4.7 Phase 7: CI/CD Pipeline

#### 4.7.1 Create GitHub Actions Workflow
File: `.github/workflows/deploy-comp-ai-service.yml`
```yaml
name: Deploy Comp AI Service to Production

on:
  push:
    branches:
      - master
      - main
    paths:
      - 'lianel/dc/comp-ai-service/**'
      - 'lianel/dc/docker-compose.comp-ai.yaml'
      - '.github/workflows/deploy-comp-ai-service.yml'
  workflow_dispatch:

env:
  REGISTRY: ghcr.io

jobs:
  build-and-deploy:
    name: Build and Deploy Comp AI Service
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write

    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to GitHub Container Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.REGISTRY }}/${{ github.repository }}/comp-ai-service
          tags: |
            type=ref,event=branch
            type=sha,prefix={{branch}}-
            type=raw,value=latest,enable={{is_default_branch}}

      - name: Build and push Docker image
        uses: docker/build-push-action@v5
        with:
          context: ./lianel/dc/comp-ai-service
          file: ./lianel/dc/comp-ai-service/Dockerfile
          platforms: linux/amd64
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

      - name: Setup SSH Key
        uses: webfactory/ssh-agent@v0.9.0
        with:
          ssh-private-key: ${{ secrets.SSH_PRIVATE_KEY }}

      - name: Deploy to Remote Host
        run: |
          ssh -F /dev/null -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no root@72.60.80.84 << 'EOF'
            cd /root/hosting-base
            git pull origin master
            cd lianel/dc
            docker compose -f docker-compose.comp-ai.yaml pull
            docker compose -f docker-compose.comp-ai.yaml up -d comp-ai-service
            docker compose -f docker-compose.comp-ai.yaml ps
          EOF
```

### 4.8 Phase 8: Environment Variables

#### 4.8.1 Add to `.env.example`
```bash
# Comp AI Service Configuration
COMP_AI_PORT=3002
COMP_AI_MODEL_PATH=/models
COMP_AI_API_KEY=your_api_key_here
COMP_AI_MAX_TOKENS=4096
COMP_AI_TEMPERATURE=0.7
COMP_AI_KEYCLOAK_CLIENT_ID=comp-ai-service
COMP_AI_KEYCLOAK_CLIENT_SECRET=your_client_secret_here
```

#### 4.8.2 Update Remote Host `.env`
- Copy new variables to remote host `.env` file
- Set actual values (secrets)

---

## 5. Deployment Execution

### 5.1 Pre-Deployment Checklist
- [ ] Service code is complete and tested
- [ ] Dockerfile builds successfully
- [ ] Docker Compose configuration is correct
- [ ] Nginx configuration is added
- [ ] Keycloak client is created
- [ ] Database schema is created (if needed)
- [ ] Environment variables are set
- [ ] Monitoring is configured
- [ ] CI/CD pipeline is set up
- [ ] Documentation is complete

### 5.2 Deployment Steps (Manual)

#### Step 1: Build and Test Locally
```bash
cd lianel/dc
docker compose -f docker-compose.comp-ai.yaml build
docker compose -f docker-compose.comp-ai.yaml up -d
docker compose -f docker-compose.comp-ai.yaml logs -f comp-ai-service
```

#### Step 2: Test Service
```bash
# Health check
curl http://localhost:3002/health

# API test (with authentication)
curl -H "Authorization: Bearer $TOKEN" http://localhost:3002/api/v1/...
```

#### Step 3: Deploy to Remote Host
```bash
# SSH to remote host
ssh -F /dev/null -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no \
  -o IdentitiesOnly=yes -i ~/.ssh/id_ed25519_host root@72.60.80.84

# On remote host
cd /root/hosting-base
git pull origin master
cd lianel/dc

# Update .env file with Comp AI variables
nano .env

# Build and start service
docker compose -f docker-compose.comp-ai.yaml build
docker compose -f docker-compose.comp-ai.yaml up -d comp-ai-service

# Verify service is running
docker compose -f docker-compose.comp-ai.yaml ps
docker logs lianel-comp-ai-service --tail 50
```

#### Step 4: Update Nginx
```bash
# On remote host
cd /root/hosting-base/lianel/dc/nginx
# Edit config/nginx.conf (add Comp AI location block)
nano config/nginx.conf

# Test Nginx configuration
docker exec lianel-nginx nginx -t

# Reload Nginx
docker exec lianel-nginx nginx -s reload
```

#### Step 5: Verify Deployment
```bash
# Test health endpoint through Nginx
curl https://www.lianel.se/api/v1/comp-ai/health

# Test API endpoint (with authentication)
curl -H "Authorization: Bearer $TOKEN" \
  https://www.lianel.se/api/v1/comp-ai/...

# Check logs
docker logs lianel-comp-ai-service --tail 100

# Check metrics
curl http://localhost:9090/api/v1/query?query=up{job="comp-ai-service"}
```

### 5.3 Post-Deployment Validation

#### 5.3.1 Functional Testing
- [ ] Health endpoint responds
- [ ] Authentication works (Keycloak)
- [ ] API endpoints are accessible
- [ ] Database connections work (if applicable)
- [ ] Error handling works correctly

#### 5.3.2 Performance Testing
- [ ] Response times are acceptable
- [ ] Service handles concurrent requests
- [ ] Memory usage is within limits
- [ ] CPU usage is reasonable

#### 5.3.3 Monitoring Validation
- [ ] Prometheus metrics are collected
- [ ] Grafana dashboard shows data
- [ ] Logs are collected by Loki
- [ ] Alerts are configured (if needed)

---

## 6. Rollback Procedure

### 6.1 Quick Rollback
```bash
# Stop service
docker compose -f docker-compose.comp-ai.yaml stop comp-ai-service

# Remove service
docker compose -f docker-compose.comp-ai.yaml rm -f comp-ai-service

# Revert Nginx config (if needed)
git checkout HEAD -- nginx/config/nginx.conf
docker exec lianel-nginx nginx -s reload
```

### 6.2 Rollback to Previous Version
```bash
# Pull previous image
docker pull ghcr.io/NimaLAN74/hosting-base/comp-ai-service:previous-tag

# Update docker-compose to use specific tag
# Edit docker-compose.comp-ai.yaml: image: ghcr.io/.../comp-ai-service:previous-tag

# Restart service
docker compose -f docker-compose.comp-ai.yaml up -d comp-ai-service
```

---

## 7. Maintenance & Operations

### 7.1 Regular Tasks
- Monitor service logs daily
- Review Grafana dashboards weekly
- Update dependencies monthly
- Review and rotate secrets quarterly

### 7.2 Scaling Considerations
- Horizontal scaling: Add more container instances behind load balancer
- Vertical scaling: Increase container resources (CPU/memory)
- Database scaling: Optimize queries, add indexes, consider read replicas

### 7.3 Backup & Recovery
- Database backups (if applicable): Daily automated backups
- Configuration backups: Git repository
- Model backups: Store models in versioned storage

---

## 8. Security Considerations

### 8.1 Authentication & Authorization
- ✅ Keycloak OAuth2/OIDC integration
- ✅ Service-to-service authentication
- ✅ Role-based access control (RBAC)

### 8.2 Network Security
- ✅ Service only exposed through Nginx
- ✅ Internal Docker network isolation
- ✅ Rate limiting on Nginx

### 8.3 Data Security
- ✅ Environment variables for secrets
- ✅ Encrypted database connections
- ✅ Input validation and sanitization
- ✅ Output encoding

### 8.4 Compliance
- ✅ GDPR compliance (if handling user data)
- ✅ Data retention policies
- ✅ Audit logging

---

## 9. Documentation Requirements

### 9.1 Service Documentation
- [ ] API documentation (OpenAPI/Swagger)
- [ ] Architecture diagram
- [ ] Configuration guide
- [ ] Troubleshooting guide
- [ ] Runbook for operations

### 9.2 User Documentation
- [ ] Getting started guide
- [ ] API usage examples
- [ ] Authentication guide
- [ ] Error code reference

---

## 10. Timeline Estimate

| Phase | Task | Estimated Time |
|-------|------|----------------|
| 1 | Service Preparation | 1-2 weeks |
| 2 | Docker Compose Setup | 1 day |
| 3 | Nginx Configuration | 1 day |
| 4 | Keycloak Integration | 1 day |
| 5 | Database Setup | 1-2 days |
| 6 | Monitoring Setup | 2-3 days |
| 7 | CI/CD Pipeline | 2-3 days |
| 8 | Testing & Validation | 3-5 days |
| **Total** | | **3-4 weeks** |

---

## 11. Success Criteria

- ✅ Service is accessible via HTTPS
- ✅ Authentication works correctly
- ✅ All API endpoints are functional
- ✅ Monitoring and logging are operational
- ✅ CI/CD pipeline deploys automatically
- ✅ Service handles expected load
- ✅ Documentation is complete

---

## 12. Next Steps

1. **Review and Approve Plan**: Review this plan with stakeholders
2. **Gather Requirements**: Clarify Comp AI service specifics (language, framework, dependencies)
3. **Create Service**: Implement Comp AI service following the plan
4. **Execute Deployment**: Follow phases 1-8 sequentially
5. **Monitor and Iterate**: Monitor service post-deployment and iterate based on feedback

---

## 13. References

- Existing service deployments:
  - Energy Service: `lianel/dc/energy-service/`
  - Profile Service: `lianel/dc/profile-service/`
- Infrastructure documentation: `lianel/docs/`
- Deployment scripts: `lianel/dc/scripts/deployment/`

---

**Document Status**: 📋 **DRAFT - Ready for Review**  
**Last Updated**: January 23, 2026  
**Next Review**: After Comp AI service requirements are clarified
