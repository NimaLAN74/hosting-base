#!/bin/bash
# Deployment script for energy service
# Usage: ./deploy-energy-service.sh <IMAGE_TAG> [GITHUB_TOKEN] [GITHUB_ACTOR]

set -euo pipefail

IMAGE_TAG="${1:-}"
GITHUB_TOKEN="${2:-}"
GITHUB_ACTOR="${3:-}"
SERVICE_NAME="energy-service"
LOCAL_TAG="lianel-energy-service:latest"
CONTAINER_NAME="lianel-energy-service"

if [ -z "$IMAGE_TAG" ]; then
  echo "❌ Error: IMAGE_TAG is required"
  echo "Usage: $0 <IMAGE_TAG> [GITHUB_TOKEN] [GITHUB_ACTOR]"
  exit 1
fi

echo "=== Deploying Energy Service ==="
echo "Image: $IMAGE_TAG"

cd /root/lianel/dc

# Clear any stale Docker auth that might interfere with public packages
docker logout ghcr.io 2>/dev/null || true

# Try to pull (packages are public, should work without auth)
echo "Pulling image..."
if docker pull "$IMAGE_TAG" 2>&1; then
  echo "✅ Image pulled successfully (public package)"
else
  PULL_EXIT=$?
  echo "⚠️  Pull failed (exit code: $PULL_EXIT), trying with authentication..."
  # If pull fails, try with auth (in case package became private or needs auth)
  if [ -n "$GITHUB_TOKEN" ] && [ -n "$GITHUB_ACTOR" ]; then
    echo "Authenticating with GitHub..."
    echo "$GITHUB_TOKEN" | docker login ghcr.io -u "$GITHUB_ACTOR" --password-stdin 2>&1 || true
    if docker pull "$IMAGE_TAG" 2>&1; then
      echo "✅ Image pulled successfully with authentication"
    else
      echo "❌ Error: Failed to pull image even with authentication"
      exit 1
    fi
  else
    echo "❌ Error: Failed to pull image and no authentication available"
    exit 1
  fi
fi

# Tag for local use
echo "Tagging image..."
docker tag "$IMAGE_TAG" "$LOCAL_TAG"

# Restart container - use backend compose file
echo "Restarting container..."
docker stop $CONTAINER_NAME 2>/dev/null || true
docker rm $CONTAINER_NAME 2>/dev/null || true

# Use docker-compose.backend.yaml (energy-service is defined there)
# Try docker compose first, fallback to docker-compose
if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
  docker compose -f docker-compose.infra.yaml -f docker-compose.backend.yaml up -d $SERVICE_NAME
elif command -v docker-compose >/dev/null 2>&1; then
  docker-compose -f docker-compose.infra.yaml -f docker-compose.backend.yaml up -d $SERVICE_NAME
else
  echo "❌ Error: Neither 'docker compose' nor 'docker-compose' found"
  exit 1
fi

# Wait a moment for container to start
sleep 5

# Verify container is running (exact name match)
if docker ps --format '{{.Names}}' | grep -q "^${CONTAINER_NAME}$"; then
  echo "✅ Container is running"
  docker ps --filter "name=${CONTAINER_NAME}" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}\t{{.Image}}"
else
  echo "❌ Container failed to start"
  echo "Checking stopped containers:"
  docker ps -a --filter "name=${CONTAINER_NAME}" || true
  echo "Recent logs:"
  docker logs ${CONTAINER_NAME} --tail 50 2>&1 || true
  exit 1
fi

echo "✅ Energy Service deployment complete!"
