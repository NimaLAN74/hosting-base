#!/bin/bash
# Run ON THE SERVER to fix 502 on /api/v1/stock-service/* (watchlist, health, status).
# Ensures lianel-stock-service is up; restarts it if needed; shows logs on failure.
# Usage: ssh server 'bash -s' < ensure-stock-service-up.sh
#    or on server: cd /root/lianel/dc && bash scripts/maintenance/ensure-stock-service-up.sh

set -e

CONTAINER="lianel-stock-service"
DC_DIR=""
for d in /root/lianel/dc /root/hosting-base/lianel/dc .; do
  if [ -f "$d/docker-compose.stock-service.yaml" ]; then
    DC_DIR="$d"
    break
  fi
done

if [ -z "$DC_DIR" ] || [ ! -d "$DC_DIR" ]; then
  echo "ERROR: docker-compose.stock-service.yaml not found"
  exit 1
fi

cd "$DC_DIR"
COMPOSE="docker compose"
docker compose version >/dev/null 2>&1 || COMPOSE="docker-compose"

echo "=== Stock service (fix 502) ==="
echo "Directory: $DC_DIR"
echo ""

if docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER}$"; then
  if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER}$"; then
    echo "Container $CONTAINER is already running."
    curl -sS -o /dev/null -w "Health: %{http_code}\n" http://localhost:3003/health 2>/dev/null || echo "Health: unreachable"
  else
    echo "Container $CONTAINER exists but is stopped. Starting..."
    $COMPOSE -f docker-compose.infra.yaml -f docker-compose.stock-service.yaml up -d stock-service
    sleep 3
    if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER}$"; then
      echo "Started. Health: $(curl -sS -o /dev/null -w '%{http_code}' http://localhost:3003/health 2>/dev/null || echo 'check')"
    else
      echo "Failed to start. Last logs:"
      docker logs "$CONTAINER" --tail 30 2>&1
      exit 1
    fi
  fi
else
  echo "Container $CONTAINER not found. Bringing up stack..."
  $COMPOSE -f docker-compose.infra.yaml -f docker-compose.stock-service.yaml up -d stock-service
  sleep 5
  if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER}$"; then
    echo "Started. Health: $(curl -sS -o /dev/null -w '%{http_code}' http://localhost:3003/health 2>/dev/null || echo 'check')"
  else
    echo "Failed to start. Logs:"
    docker logs "$CONTAINER" --tail 50 2>&1 || true
    exit 1
  fi
fi

echo ""
echo "If you still see 502 from the browser, check:"
echo "  1. Nginx and this host are on the same Docker network (lianel-network)"
echo "  2. docker logs $CONTAINER --tail 100"
