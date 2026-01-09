#!/bin/bash
# Script to check and restart energy service if needed

set -e

CONTAINER_NAME="lianel-energy-service"
COMPOSE_FILE="docker-compose.backend.yaml"

echo "🔍 Checking energy service status..."

# Check if container exists
if ! docker ps -a --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "❌ Container ${CONTAINER_NAME} does not exist"
    echo "📦 Starting energy service..."
    cd /root/lianel/dc
    docker compose -f ${COMPOSE_FILE} up -d energy-service
    echo "⏳ Waiting for service to start..."
    sleep 5
else
    # Check if container is running
    if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
        echo "⚠️  Container ${CONTAINER_NAME} exists but is not running"
        echo "🔄 Restarting container..."
        docker start ${CONTAINER_NAME}
        echo "⏳ Waiting for service to start..."
        sleep 5
    else
        echo "✅ Container ${CONTAINER_NAME} is running"
    fi
fi

# Check container health
echo ""
echo "🏥 Checking container health..."
CONTAINER_STATUS=$(docker inspect --format='{{.State.Status}}' ${CONTAINER_NAME} 2>/dev/null || echo "not_found")
echo "   Status: ${CONTAINER_STATUS}"

if [ "${CONTAINER_STATUS}" != "running" ]; then
    echo "❌ Container is not running. Attempting to start..."
    cd /root/lianel/dc
    docker compose -f ${COMPOSE_FILE} up -d energy-service
    sleep 5
fi

# Check if service is responding
echo ""
echo "🌐 Testing service health endpoint..."
HEALTH_RESPONSE=$(docker exec ${CONTAINER_NAME} curl -s http://localhost:3001/health 2>/dev/null || echo "failed")
if echo "${HEALTH_RESPONSE}" | grep -q "ok"; then
    echo "✅ Service is healthy"
else
    echo "⚠️  Service health check failed"
    echo "   Response: ${HEALTH_RESPONSE}"
    echo ""
    echo "📋 Container logs (last 20 lines):"
    docker logs --tail 20 ${CONTAINER_NAME} 2>&1 || true
fi

# Check network connectivity
echo ""
echo "🔗 Checking network connectivity..."
if docker exec ${CONTAINER_NAME} ping -c 1 keycloak >/dev/null 2>&1; then
    echo "✅ Can reach keycloak"
else
    echo "⚠️  Cannot reach keycloak"
fi

# Check if container is on correct network
echo ""
echo "🌐 Checking network membership..."
NETWORKS=$(docker inspect --format='{{range $net, $conf := .NetworkSettings.Networks}}{{$net}} {{end}}' ${CONTAINER_NAME} 2>/dev/null || echo "none")
echo "   Networks: ${NETWORKS}"
if echo "${NETWORKS}" | grep -q "lianel-network"; then
    echo "✅ Container is on lianel-network"
else
    echo "⚠️  Container is NOT on lianel-network"
    echo "🔧 Connecting to lianel-network..."
    docker network connect lianel-network ${CONTAINER_NAME} 2>/dev/null || true
fi

echo ""
echo "✅ Energy service check complete"
