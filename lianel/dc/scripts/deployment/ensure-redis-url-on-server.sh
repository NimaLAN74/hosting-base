#!/usr/bin/env bash
# One-time on server: set REDIS_URL in .env so stock-service can cache today's prices.
# Run from repo root on the server, or via: ssh server 'bash -s' < scripts/deployment/ensure-redis-url-on-server.sh
# Requires: REDIS_PASSWORD in .env (same as Airflow Redis). Redis must be on lianel-network (docker-compose.airflow.yaml).
set -euo pipefail

DC_DIR="${DC_DIR:-/root/lianel/dc}"
for d in /root/lianel/dc /root/hosting-base/lianel/dc; do
  [ -f "$d/.env" ] && DC_DIR="$d" && break
done
ENV_FILE="$DC_DIR/.env"
[ -f "$ENV_FILE" ] || { echo "❌ .env not found at $ENV_FILE"; exit 1; }

# REDIS_PASSWORD from .env (same as Airflow)
source "$ENV_FILE" 2>/dev/null || true
REDIS_PASSWORD="${REDIS_PASSWORD:-}"
if [ -z "$REDIS_PASSWORD" ]; then
  echo "⚠️ REDIS_PASSWORD not set in .env; cannot build REDIS_URL. Add REDIS_PASSWORD to .env and re-run."
  exit 1
fi

# Redis container name (Compose project may be dc or folder name)
REDIS_HOST="${REDIS_HOST:-}"
if [ -z "$REDIS_HOST" ]; then
  for name in redis dc_redis_1 dc-redis-1 lianel_redis_1; do
    if docker ps --format '{{.Names}}' 2>/dev/null | grep -q "^${name}$"; then
      REDIS_HOST="$name"
      break
    fi
  done
fi
[ -z "$REDIS_HOST" ] && REDIS_HOST="redis"
# Use DB 1 (Celery uses 0)
REDIS_URL="redis://:${REDIS_PASSWORD}@${REDIS_HOST}:6379/1"

if grep -q '^REDIS_URL=' "$ENV_FILE" 2>/dev/null; then
  echo "REDIS_URL already set in $ENV_FILE"
else
  echo "REDIS_URL=$REDIS_URL" >> "$ENV_FILE"
  echo "✅ Appended REDIS_URL to $ENV_FILE"
fi

echo "Restarting stock-service to pick up REDIS_URL..."
cd "$DC_DIR"
docker compose -f docker-compose.infra.yaml -f docker-compose.stock-service.yaml up -d --force-recreate --no-deps stock-service 2>/dev/null || \
  docker compose -f docker-compose.stock-service.yaml up -d --force-recreate --no-deps stock-service 2>/dev/null || true
echo "✅ Done. Today's prices will be cached in Redis."
