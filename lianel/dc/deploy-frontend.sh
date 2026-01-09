#!/bin/bash
# Deployment script for frontend service
# This script is maintained in the repository and copied to remote host during deployment
# Usage: ./deploy-frontend.sh <IMAGE_TAG>

set -euo pipefail

IMAGE_TAG="${1:-}"
SERVICE_NAME="frontend"
LOCAL_TAG="lianel-frontend:latest"

if [ -z "$IMAGE_TAG" ]; then
  echo "❌ Error: IMAGE_TAG is required"
  echo "Usage: $0 <IMAGE_TAG>"
  exit 1
fi

echo "=== Deploying Frontend ==="
echo "Image: $IMAGE_TAG"

cd /root/lianel/dc

# Remove old local tags to force fresh pull
# This ensures we always get the latest image, not a cached version
echo "Removing old local image tags to force fresh pull..."
docker rmi "$LOCAL_TAG" 2>/dev/null || true
docker rmi "$IMAGE_TAG" 2>/dev/null || true

# Clear any stale Docker auth
docker logout ghcr.io 2>/dev/null || true

# Authenticate if credentials provided
if [ -n "${GITHUB_TOKEN:-}" ] && [ -n "${GITHUB_ACTOR:-}" ]; then
  echo "Authenticating with GitHub..."
  echo "$GITHUB_TOKEN" | docker login ghcr.io -u "$GITHUB_ACTOR" --password-stdin 2>&1 || true
fi

# Force pull the latest image (always pull, even if local copy exists)
echo "Pulling latest image (forcing update)..."
if docker pull "$IMAGE_TAG" 2>&1; then
  echo "✅ Image pulled successfully"
else
  PULL_EXIT=$?
  echo "❌ Error: Failed to pull image (exit code: $PULL_EXIT)"
  echo "Image tag: $IMAGE_TAG"
  echo "Checking if package is public or authentication is needed..."
  exit 1
fi

# Tag for local use
echo "Tagging image for local use..."
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

# Verify deployment
echo "Waiting for container to start..."
sleep 5
if docker ps --format '{{.Names}}' | grep -q "^lianel-$SERVICE_NAME$"; then
  echo "✅ Deployment successful"
  docker ps --filter "name=lianel-$SERVICE_NAME" --format "table {{.Names}}\t{{.Status}}\t{{.Image}}"
  echo ""
  echo "Image details:"
  docker inspect "$IMAGE_TAG" --format 'Digest: {{index .RepoDigests 0}}' 2>/dev/null || echo "Digest not available"
else
  echo "❌ Container not running"
  echo "Checking stopped containers:"
  docker ps -a --filter "name=lianel-$SERVICE_NAME" || true
  echo "Recent logs:"
  docker logs lianel-$SERVICE_NAME --tail 20 2>&1 || true
  exit 1
fi

# Cleanup old images
echo "Cleaning up old images..."
docker image prune -f

echo "=== Deployment Complete ==="
