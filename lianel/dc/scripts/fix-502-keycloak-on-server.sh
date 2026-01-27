#!/bin/bash
# Fix 502 on https://www.lianel.se/auth/ — run this ON THE SERVER where nginx and Keycloak run.
# Cause: nginx cannot reach keycloak:8080 (different Docker networks or keycloak not running).
# Fix: start both keycloak and nginx from docker-compose.infra.yaml so they share lianel-network.

set -e

DC_DIR=""
for d in /root/lianel/dc /root/hosting-base/lianel/dc .; do
  if [ -f "$d/docker-compose.infra.yaml" ]; then
    DC_DIR="$d"
    break
  fi
done

if [ -z "$DC_DIR" ] || [ ! -d "$DC_DIR" ]; then
  echo "ERROR: docker-compose.infra.yaml not found in /root/lianel/dc or /root/hosting-base/lianel/dc"
  exit 1
fi

echo "Using directory: $DC_DIR"
cd "$DC_DIR"

echo "Starting keycloak + nginx from docker-compose.infra.yaml (lianel-network)..."
echo "If you see 'port already in use', another nginx may be running; stop it (docker stop nginx-proxy) and run this script again."
if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
  docker compose -f docker-compose.infra.yaml up -d
elif command -v docker-compose >/dev/null 2>&1; then
  docker-compose -f docker-compose.infra.yaml up -d
else
  echo "ERROR: docker compose or docker-compose not found"
  exit 1
fi

echo "Waiting for Keycloak to be ready..."
for i in 1 2 3 4 5 6 7 8 9 10; do
  if docker ps --filter name=keycloak --filter status=running -q | grep -q .; then
    echo "Keycloak container is running."
    break
  fi
  sleep 5
done

sleep 10
docker ps --filter name=keycloak
docker ps --filter name=nginx-proxy

echo "Reloading nginx..."
docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload && echo "✅ Nginx reloaded" || echo "⚠️ Nginx reload failed"

echo ""
echo "Test: curl -s -o /dev/null -w '%{http_code}' 'https://www.lianel.se/auth/realms/lianel/protocol/openid-connect/auth?client_id=frontend-client&redirect_uri=https://www.lianel.se/&response_type=code&scope=openid'"
echo "Expected: 302 or 200 (not 502)"
