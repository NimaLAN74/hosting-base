#!/bin/bash
# Quick verification commands for energy service
# Run this on your local machine: ./verify-energy-remote.sh

REMOTE_HOST="72.60.80.84"
REMOTE_USER="root"

echo "=========================================="
echo "Energy Service Remote Verification"
echo "=========================================="
echo ""

echo "1️⃣  Container Status:"
ssh ${REMOTE_USER}@${REMOTE_HOST} "docker ps --filter 'name=lianel-energy-service' --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}\t{{.Image}}'"

echo ""
echo "2️⃣  Container Logs (last 20 lines):"
ssh ${REMOTE_USER}@${REMOTE_HOST} "docker logs lianel-energy-service --tail 20"

echo ""
echo "3️⃣  Internal Health Check:"
ssh ${REMOTE_USER}@${REMOTE_HOST} "docker exec lianel-energy-service wget -q -O- http://localhost:3001/health 2>/dev/null || echo 'Health check failed'"

echo ""
echo "4️⃣  Network Connectivity:"
ssh ${REMOTE_USER}@${REMOTE_HOST} "docker inspect lianel-energy-service --format '{{range \$net, \$conf := .NetworkSettings.Networks}}{{\$net}}{{end}}' 2>/dev/null || echo 'Network check failed'"

echo ""
echo "5️⃣  Resource Usage:"
ssh ${REMOTE_USER}@${REMOTE_HOST} "docker stats lianel-energy-service --no-stream --format 'table {{.Container}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}' 2>/dev/null || echo 'Stats unavailable'"

echo ""
echo "6️⃣  External Endpoint Test:"
curl -s -o /dev/null -w "HTTP Status: %{http_code}\n" https://www.lianel.se/api/energy/health --insecure 2>/dev/null || echo "External endpoint test failed"

echo ""
echo "=========================================="
echo "Verification Complete"
echo "=========================================="
