#!/bin/bash
set -euo pipefail

IMAGE_TAG="$1"
SERVICE_NAME="frontend"
LOCAL_TAG="lianel-frontend:latest"

echo "=== Deploying Frontend ==="
echo "Image: $IMAGE_TAG"

cd /root/lianel/dc

# Clear any stale Docker auth
docker logout ghcr.io 2>/dev/null || true

# Try to pull
echo "Pulling image..."
if docker pull "$IMAGE_TAG" 2>&1; then
  echo "✅ Image pulled successfully"
else
  PULL_EXIT=$?
  echo "⚠️  Pull failed (exit code: $PULL_EXIT), trying with authentication..."
  if [ -n "${GITHUB_TOKEN:-}" ] && [ -n "${GITHUB_ACTOR:-}" ]; then
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

# Restart container - use docker compose (newer) or docker-compose (older)
echo "Restarting container..."
docker stop lianel-$SERVICE_NAME 2>/dev/null || true
docker rm lianel-$SERVICE_NAME 2>/dev/null || true

# Try docker compose first, fallback to docker-compose
if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
  docker compose -f docker-compose.yaml up -d $SERVICE_NAME
elif command -v docker-compose >/dev/null 2>&1; then
  docker-compose -f docker-compose.yaml up -d $SERVICE_NAME
else
  echo "❌ Error: Neither 'docker compose' nor 'docker-compose' found"
  exit 1
fi

# Verify
sleep 5
if docker ps --format '{{.Names}}' | grep -q "^lianel-$SERVICE_NAME$"; then
  echo "✅ Deployment successful"
  docker ps --filter "name=lianel-$SERVICE_NAME" --format "table {{.Names}}\t{{.Status}}\t{{.Image}}"
else
  echo "❌ Container not running"
  echo "Checking stopped containers:"
  docker ps -a --filter "name=lianel-$SERVICE_NAME" || true
  echo "Recent logs:"
  docker logs lianel-$SERVICE_NAME --tail 20 2>&1 || true
  exit 1
fi

# Cleanup
docker image prune -f

echo "=== Done ==="
