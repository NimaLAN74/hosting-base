#!/bin/bash
# Quick script to restart energy service

set -e

cd /root/lianel/dc

echo "🔄 Restarting energy service..."

# Stop and remove container
docker stop lianel-energy-service 2>/dev/null || true
docker rm lianel-energy-service 2>/dev/null || true

# Start using docker compose
if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
  docker compose -f docker-compose.backend.yaml up -d energy-service
elif command -v docker-compose >/dev/null 2>&1; then
  docker-compose -f docker-compose.backend.yaml up -d energy-service
else
  echo "❌ Error: Neither 'docker compose' nor 'docker-compose' found"
  exit 1
fi

# Wait for service to start
sleep 5

# Check status
if docker ps --format '{{.Names}}' | grep -q "^lianel-energy-service$"; then
  echo "✅ Energy service is running"
  docker ps --filter "name=lianel-energy-service" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
else
  echo "❌ Energy service failed to start"
  echo "📋 Container logs:"
  docker logs lianel-energy-service --tail 30 2>&1 || true
  exit 1
fi
