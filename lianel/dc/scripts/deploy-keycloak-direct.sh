#!/bin/bash
# Complete deployment script for Keycloak direct integration
# This script:
# 1. Configures Keycloak with proper clients and MFA
# 2. Updates Nginx configuration
# 3. Deploys frontend and backend
# 4. Configures Grafana

set -e

REMOTE_HOST="${REMOTE_HOST:-72.60.80.84}"
REMOTE_USER="${REMOTE_USER:-root}"
REPO_DIR="${REPO_DIR:-/root/hosting-base}"

echo "=== Keycloak Direct Integration Deployment ==="
echo "Remote Host: $REMOTE_HOST"
echo ""

# Step 1: Configure Keycloak
echo "Step 1: Configuring Keycloak..."
ssh ${REMOTE_USER}@${REMOTE_HOST} "cd ${REPO_DIR}/lianel/dc && bash scripts/keycloak-setup/configure-keycloak-direct.sh"

# Step 2: Update Nginx configuration (remove OAuth2-proxy, add Keycloak endpoints)
echo ""
echo "Step 2: Updating Nginx configuration..."
# Nginx config will be updated via git push

# Step 3: Deploy frontend
echo ""
echo "Step 3: Deploying frontend..."
ssh ${REMOTE_USER}@${REMOTE_HOST} "cd ${REPO_DIR}/lianel/dc && docker compose -f docker-compose.frontend.yaml build frontend && docker compose -f docker-compose.frontend.yaml up -d frontend"

# Step 4: Deploy backend
echo ""
echo "Step 4: Deploying backend..."
ssh ${REMOTE_USER}@${REMOTE_HOST} "cd ${REPO_DIR}/lianel/dc && docker compose -f docker-compose.infra.yaml -f docker-compose.backend.yaml build profile-service && docker compose -f docker-compose.infra.yaml -f docker-compose.backend.yaml up -d profile-service"

# Step 5: Configure Grafana
echo ""
echo "Step 5: Configuring Grafana..."
# Grafana config will be updated via git push

# Step 6: Restart Nginx
echo ""
echo "Step 6: Restarting Nginx..."
ssh ${REMOTE_USER}@${REMOTE_HOST} "docker restart lianel-nginx"

echo ""
echo "=== Deployment Complete ==="
echo ""
echo "Next steps:"
echo "1. Test login flow: https://www.lianel.se"
echo "2. Verify MFA prompts"
echo "3. Test logout"
echo "4. Test API access with tokens"
echo "5. Verify Grafana SSO"

