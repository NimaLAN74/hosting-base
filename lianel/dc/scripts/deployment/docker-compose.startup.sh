#!/bin/bash
# Script to properly start all services with the correct network configuration

cd "$(dirname "$0")" || exit 1

echo "Starting lianel stack with all services..."

# Ensure the network exists first
docker network create lianel-network 2>/dev/null || true

# Start infrastructure (keycloak, nginx) 
echo "Starting infrastructure..."
docker compose -f docker-compose.infra.yaml up -d

# Start frontend
echo "Starting frontend..."
docker compose -f docker-compose.yaml up -d frontend

# Fix network: docker-compose creates a default network, but we need lianel-network
sleep 2
echo "Fixing frontend network configuration..."
docker network disconnect dc_default lianel-frontend 2>/dev/null || true
docker network connect lianel-network lianel-frontend 2>/dev/null || true

# Start backend services (profile and energy)
echo "Starting backend services..."
docker compose -f docker-compose.backend.yaml up -d

# Start monitoring
echo "Starting monitoring..."
docker compose -f docker-compose.monitoring.yaml up -d

# Start airflow
echo "Starting airflow..."
docker compose -f docker-compose.airflow.yaml up -d

# Start oauth2-proxy
echo "Starting oauth2-proxy..."
docker compose -f docker-compose.oauth2-proxy.yaml up -d

echo ""
echo "✅ All services started!"
echo ""
echo "Services are available at:"
echo "  🌐 Frontend: https://lianel.se"
echo "  🔐 Keycloak: https://auth.lianel.se"
echo "  📊 Airflow: https://lianel.se/airflow"
echo "  📈 Grafana: https://lianel.se/grafana"

