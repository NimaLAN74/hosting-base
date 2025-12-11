#!/bin/bash
# Complete deployment script for Keycloak direct integration - REMOTE HOST ONLY
# This script runs entirely on the remote host via SSH
# Usage: ssh root@72.60.80.84 'bash -s' < deploy-keycloak-direct-remote.sh

set -e

REPO_DIR="${REPO_DIR:-/root/hosting-base}"
KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
REALM_NAME="${REALM_NAME:-lianel}"

echo "=== Keycloak Direct Integration Deployment (Remote Host) ==="
echo "Repository: $REPO_DIR"
echo ""

# Step 1: Pull latest code
echo "Step 1: Pulling latest code..."
cd "$REPO_DIR" || exit 1
git pull origin main || git pull origin master || echo "Warning: Could not pull latest code"

# Step 2: Configure Keycloak
echo ""
echo "Step 2: Configuring Keycloak..."
cd "$REPO_DIR/lianel/dc" || exit 1
bash scripts/keycloak-setup/configure-keycloak-direct.sh

# Step 3: Build and deploy frontend
echo ""
echo "Step 3: Building and deploying frontend..."
cd "$REPO_DIR/lianel/dc" || exit 1
docker compose -f docker-compose.frontend.yaml build frontend
docker stop lianel-frontend 2>/dev/null || true
docker rm lianel-frontend 2>/dev/null || true
docker compose -f docker-compose.frontend.yaml up -d frontend

# Step 4: Build and deploy backend
echo ""
echo "Step 4: Building and deploying backend..."
cd "$REPO_DIR/lianel/dc" || exit 1
docker compose -f docker-compose.infra.yaml -f docker-compose.backend.yaml build profile-service
docker stop lianel-profile-service 2>/dev/null || true
docker rm lianel-profile-service 2>/dev/null || true
docker compose -f docker-compose.infra.yaml -f docker-compose.backend.yaml up -d profile-service

# Step 5: Update Grafana configuration
echo ""
echo "Step 5: Updating Grafana configuration..."
cd "$REPO_DIR/lianel/dc" || exit 1
docker compose -f docker-compose.monitoring.yaml up -d grafana

# Step 6: Update Nginx configuration
echo ""
echo "Step 6: Updating Nginx configuration..."
docker cp "$REPO_DIR/lianel/dc/nginx/config/nginx.conf" lianel-nginx:/etc/nginx/nginx.conf
docker exec lianel-nginx nginx -t
docker restart lianel-nginx

echo ""
echo "=== Deployment Complete ==="
echo ""
echo "Services deployed:"
echo "- Frontend: https://www.lianel.se"
echo "- Backend API: https://www.lianel.se/api/"
echo "- Keycloak: https://auth.lianel.se"
echo "- Grafana: https://monitoring.lianel.se"
echo ""
echo "Next: Test the complete flow:"
echo "1. Visit https://www.lianel.se"
echo "2. Should redirect to Keycloak login"
echo "3. Enter credentials → MFA prompt"
echo "4. After login, test logout"
echo "5. Test API access"

