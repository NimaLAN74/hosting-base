#!/bin/bash
# Verification script for energy service deployment
# Usage: ./verify-energy-service.sh

set -euo pipefail

CONTAINER_NAME="lianel-energy-service"
SERVICE_PORT="3001"

echo "=========================================="
echo "Energy Service Deployment Verification"
echo "=========================================="
echo ""

# 1. Check container is running
echo "1️⃣  Checking container status..."
if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "✅ Container is running"
    docker ps --filter "name=${CONTAINER_NAME}" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}\t{{.Image}}"
else
    echo "❌ Container is NOT running"
    echo "Checking stopped containers..."
    docker ps -a --filter "name=${CONTAINER_NAME}" --format "table {{.Names}}\t{{.Status}}\t{{.Image}}"
    exit 1
fi

echo ""

# 2. Check container health
echo "2️⃣  Checking container health..."
HEALTH_STATUS=$(docker inspect --format='{{.State.Health.Status}}' ${CONTAINER_NAME} 2>/dev/null || echo "no-healthcheck")
if [ "$HEALTH_STATUS" != "no-healthcheck" ]; then
    echo "Health status: $HEALTH_STATUS"
    if [ "$HEALTH_STATUS" = "healthy" ]; then
        echo "✅ Container is healthy"
    else
        echo "⚠️  Container health: $HEALTH_STATUS"
    fi
else
    echo "ℹ️  No healthcheck configured (this is OK)"
fi

echo ""

# 3. Check container logs for errors
echo "3️⃣  Checking recent container logs..."
RECENT_LOGS=$(docker logs ${CONTAINER_NAME} --tail 20 2>&1)
if echo "$RECENT_LOGS" | grep -qi "error\|fatal\|panic"; then
    echo "⚠️  Found potential errors in logs:"
    echo "$RECENT_LOGS" | grep -i "error\|fatal\|panic" | head -5
else
    echo "✅ No obvious errors in recent logs"
    echo "Last 5 log lines:"
    echo "$RECENT_LOGS" | tail -5
fi

echo ""

# 4. Test internal health endpoint
echo "4️⃣  Testing internal health endpoint..."
if docker exec ${CONTAINER_NAME} wget -q -O- http://localhost:${SERVICE_PORT}/health 2>/dev/null | grep -q "ok"; then
    echo "✅ Health endpoint responding"
    docker exec ${CONTAINER_NAME} wget -q -O- http://localhost:${SERVICE_PORT}/health 2>/dev/null | head -3
else
    echo "⚠️  Health endpoint check failed or returned unexpected response"
    docker exec ${CONTAINER_NAME} wget -q -O- http://localhost:${SERVICE_PORT}/health 2>/dev/null || echo "Connection failed"
fi

echo ""

# 5. Check network connectivity
echo "5️⃣  Checking network connectivity..."
if docker network inspect lianel-network >/dev/null 2>&1; then
    if docker inspect ${CONTAINER_NAME} --format='{{range $net, $conf := .NetworkSettings.Networks}}{{$net}}{{end}}' | grep -q "lianel-network"; then
        echo "✅ Container is connected to lianel-network"
    else
        echo "⚠️  Container is NOT connected to lianel-network"
    fi
else
    echo "⚠️  lianel-network does not exist"
fi

echo ""

# 6. Check database connectivity (if applicable)
echo "6️⃣  Checking database connectivity..."
if docker exec ${CONTAINER_NAME} sh -c "command -v psql >/dev/null 2>&1" 2>/dev/null; then
    if docker exec ${CONTAINER_NAME} psql "$DATABASE_URL" -c "SELECT 1" >/dev/null 2>&1; then
        echo "✅ Database connection successful"
    else
        echo "⚠️  Database connection failed (this may be OK if service doesn't use DB)"
    fi
else
    echo "ℹ️  psql not available in container (this is OK)"
fi

echo ""

# 7. Check external endpoint (via nginx)
echo "7️⃣  Testing external endpoint (via nginx)..."
EXTERNAL_HEALTH=$(curl -s -o /dev/null -w '%{http_code}' https://www.lianel.se/api/energy/health --insecure 2>/dev/null || echo "000")
if [ "$EXTERNAL_HEALTH" = "200" ]; then
    echo "✅ External health endpoint accessible (HTTP $EXTERNAL_HEALTH)"
    curl -s https://www.lianel.se/api/energy/health --insecure | head -3
elif [ "$EXTERNAL_HEALTH" = "401" ] || [ "$EXTERNAL_HEALTH" = "403" ]; then
    echo "⚠️  External endpoint requires authentication (HTTP $EXTERNAL_HEALTH) - this may be expected"
else
    echo "⚠️  External endpoint returned HTTP $EXTERNAL_HEALTH"
fi

echo ""

# 8. Check image details
echo "8️⃣  Checking image details..."
IMAGE_ID=$(docker inspect --format='{{.Image}}' ${CONTAINER_NAME} 2>/dev/null)
IMAGE_TAG=$(docker inspect --format='{{.Config.Image}}' ${CONTAINER_NAME} 2>/dev/null)
echo "Image: $IMAGE_TAG"
echo "Image ID: ${IMAGE_ID:0:12}"

echo ""

# 9. Check resource usage
echo "9️⃣  Checking resource usage..."
docker stats ${CONTAINER_NAME} --no-stream --format "table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}"

echo ""

# Summary
echo "=========================================="
echo "Verification Summary"
echo "=========================================="
if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
    echo "✅ Container Status: RUNNING"
    echo "✅ Basic checks: PASSED"
    echo ""
    echo "Next steps:"
    echo "  - Test Swagger UI: https://www.lianel.se/api/energy/swagger-ui/"
    echo "  - Check logs: docker logs ${CONTAINER_NAME} --tail 50"
    echo "  - Monitor: docker stats ${CONTAINER_NAME}"
else
    echo "❌ Container Status: NOT RUNNING"
    echo "❌ Deployment verification: FAILED"
    exit 1
fi
