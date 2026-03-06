#!/bin/bash
# Run ON THE SERVER to ensure all services (stock, comp-ai, monitoring, airflow, infra) are up.
# Brings up stacks in dependency order and reports status.
# Usage: bash ensure-all-services-up-on-server.sh

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

COMPOSE="docker compose"
command -v docker >/dev/null 2>&1 || { echo "ERROR: docker not found"; exit 1; }
docker compose version >/dev/null 2>&1 || COMPOSE="docker-compose"

echo ""
echo "=== 1. Ensure network ==="
docker network create lianel-network 2>/dev/null || true

echo ""
echo "=== 2. Infra (keycloak, nginx) ==="
$COMPOSE -f docker-compose.infra.yaml up -d
sleep 3

echo ""
echo "=== 3. Monitoring (loki, promtail, prometheus, grafana, cadvisor, node-exporter) ==="
if [ -f docker-compose.monitoring.yaml ]; then
  $COMPOSE -f docker-compose.monitoring.yaml up -d
  sleep 2
else
  echo "  (docker-compose.monitoring.yaml not found, skip)"
fi

echo ""
echo "=== 4. Airflow (redis, scheduler, worker, apiserver, triggerer, flower) ==="
if [ -f docker-compose.airflow.yaml ]; then
  $COMPOSE -f docker-compose.airflow.yaml up -d
  sleep 5
else
  echo "  (docker-compose.airflow.yaml not found, skip)"
fi

echo ""
echo "=== 5. Stock monitoring (backend + UI) ==="
if [ -f docker-compose.stock-service.yaml ]; then
  $COMPOSE -f docker-compose.infra.yaml -f docker-compose.stock-service.yaml up -d
  sleep 2
else
  echo "  (docker-compose.stock-service.yaml not found, skip)"
fi

echo ""
echo "=== 6. Comp-AI service ==="
if [ -f docker-compose.comp-ai.yaml ]; then
  $COMPOSE -f docker-compose.infra.yaml -f docker-compose.comp-ai.yaml up -d comp-ai-service
  sleep 2
else
  echo "  (docker-compose.comp-ai.yaml not found, skip)"
fi

echo ""
echo "=== 7. Frontend (main app) ==="
if [ -f docker-compose.frontend.yaml ]; then
  $COMPOSE -f docker-compose.infra.yaml -f docker-compose.frontend.yaml up -d
elif [ -f docker-compose.yaml ]; then
  $COMPOSE -f docker-compose.yaml up -d frontend 2>/dev/null || true
fi
sleep 2

echo ""
echo "=== 8. Backend (profile-service) ==="
if [ -f docker-compose.backend.yaml ]; then
  $COMPOSE -f docker-compose.infra.yaml -f docker-compose.backend.yaml up -d
  sleep 2
fi

echo ""
echo "=== 9. OAuth2 proxy ==="
if [ -f docker-compose.oauth2-proxy.yaml ]; then
  $COMPOSE -f docker-compose.oauth2-proxy.yaml up -d
fi

echo ""
echo "=== 10. Restart nginx (so upstreams resolve after all backends are up) ==="
docker restart nginx-proxy 2>/dev/null && sleep 3 && echo "  Nginx restarted" || echo "  (nginx restart skipped)"
docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload 2>/dev/null && echo "  Nginx config OK" || true

echo ""
echo "=== Container status (expected: stock, comp-ai, monitoring, airflow, infra) ==="
docker ps -a --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}" | head -60

echo ""
echo "=== Summary: key containers ==="
for name in keycloak nginx-proxy lianel-stock-service lianel-comp-ai-service prometheus grafana cadvisor loki promtail; do
  status=$(docker ps -a --filter "name=${name}" --format "{{.Names}}\t{{.Status}}" 2>/dev/null | head -1)
  if [ -n "$status" ]; then
    n=$(echo "$status" | cut -f1); s=$(echo "$status" | cut -f2-)
    if echo "$s" | grep -q "Up"; then echo "  OK   $n"; else echo "  DOWN $n ($s)"; fi
  fi
done
# Airflow containers (names may have project prefix e.g. dc-airflow-scheduler-1)
docker ps -a --filter "name=airflow" --format "{{.Names}}\t{{.Status}}" 2>/dev/null | while read -r line; do
  n=$(echo "$line" | cut -f1); s=$(echo "$line" | cut -f2-)
  [ -z "$n" ] && continue
  if echo "$s" | grep -q "Up"; then echo "  OK   $n"; else echo "  DOWN $n ($s)"; fi
done
# Redis (for Airflow)
docker ps -a --filter "name=redis" --format "{{.Names}}\t{{.Status}}" 2>/dev/null | head -1 | while read -r line; do
  n=$(echo "$line" | cut -f1); s=$(echo "$line" | cut -f2-)
  [ -n "$n" ] && (echo "$s" | grep -q "Up" && echo "  OK   $n" || echo "  DOWN $n ($s)")
done

echo ""
echo "Done."
