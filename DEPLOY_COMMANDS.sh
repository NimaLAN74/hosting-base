#!/bin/bash
# Systematic deployment of all services
# Run this on the remote host as: bash /root/DEPLOY_COMMANDS.sh

set -euxo pipefail

WORK="/root/lianel/dc"
cd "$WORK"

echo "=========================================="
echo "STEP 1: Deploy Infrastructure (Infra)"
echo "=========================================="
echo "Services: postgres, keycloak, nginx"
docker compose -f docker-compose.infra.yaml down --remove-orphans 2>/dev/null || true
docker compose -f docker-compose.infra.yaml up -d
sleep 20

echo ""
echo "Checking Postgres health..."
docker exec postgres pg_isready -U keycloak -d keycloak || echo "Postgres still starting..."

echo ""
echo "Checking Keycloak health..."
docker exec keycloak curl -s -o /dev/null -w '%{http_code}\n' http://localhost:8080/health/ready || echo "Keycloak still starting..."

sleep 10

echo ""
echo "=========================================="
echo "STEP 2: Deploy OAuth2-Proxy"
echo "=========================================="
echo "Services: oauth2-proxy"
docker compose -f docker-compose.oauth2-proxy.yaml down --remove-orphans 2>/dev/null || true
docker compose -f docker-compose.oauth2-proxy.yaml up -d
sleep 5

echo ""
echo "=========================================="
echo "STEP 3: Deploy Main Services"
echo "=========================================="
echo "Services: frontend, profile-service (uses profile-service:3000 from compose.yaml)"
docker compose -f docker-compose.yaml down --remove-orphans 2>/dev/null || true
docker compose -f docker-compose.yaml up -d
sleep 10

echo ""
echo "=========================================="
echo "STEP 4: Network Verification"
echo "=========================================="
echo "All containers on lianel-network:"
docker ps --format 'table {{.Names}}\t{{.Status}}\t{{.Networks}}'

echo ""
echo "=========================================="
echo "STEP 5: Health Checks"
echo "=========================================="

echo "Postgres health:"
docker exec postgres pg_isready -U keycloak -d keycloak && echo "✓ Postgres OK" || echo "✗ Postgres FAILED"

echo ""
echo "Keycloak health:"
docker exec keycloak curl -s -o /dev/null -w 'HTTP %{http_code}\n' http://localhost:8080/health/ready && echo "✓ Keycloak OK" || echo "✗ Keycloak FAILED"

echo ""
echo "Keycloak well-known (via nginx):"
curl -s -o /dev/null -w 'HTTP %{http_code}\n' https://auth.lianel.se/realms/lianel/.well-known/openid-configuration --insecure || echo "✗ Well-known FAILED"

echo ""
echo "Frontend health:"
curl -s -o /dev/null -w 'HTTP %{http_code}\n' https://lianel.se/health --insecure || echo "✗ Frontend FAILED"

echo ""
echo "Profile Service health:"
curl -s -o /dev/null -w 'HTTP %{http_code}\n' http://lianel-profile-service:3000/health || echo "✗ Profile Service FAILED"

echo ""
echo "Profile Service admin check (expect 401):"
curl -s -o /dev/null -w 'HTTP %{http_code}\n' http://lianel-profile-service:3000/api/admin/check || echo "✗ Admin Check FAILED"

echo ""
echo "=========================================="
echo "DEPLOYMENT COMPLETE"
echo "=========================================="
echo ""
echo "Admin Credentials:"
echo "  Username: admin"
echo "  Password: D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA"
echo "  Realm: lianel"
echo ""
echo "Test Login Flow:"
echo "  1. Visit https://lianel.se"
echo "  2. Click login"
echo "  3. Should redirect to https://auth.lianel.se"
echo "  4. Login with above credentials"
echo "  5. Should redirect back to https://lianel.se dashboard"
