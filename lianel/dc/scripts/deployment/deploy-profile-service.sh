#!/bin/bash
# Deploy profile service. Copied to remote by deploy-profile-service.yml.
# Usage: ./deploy-profile-service.sh <IMAGE_TAG>
set -euo pipefail

IMAGE_TAG="${1:-}"
SERVICE_NAME="profile-service"
LOCAL_TAG="lianel-profile-service:latest"

if [ -z "$IMAGE_TAG" ]; then
  echo "❌ Error: IMAGE_TAG is required"
  echo "Usage: $0 <IMAGE_TAG>"
  exit 1
fi

echo "=== Deploying Profile Service ==="
echo "Image: $IMAGE_TAG"

DC_DIR="${DC_DIR:-}"
if [ -z "$DC_DIR" ]; then
  for d in /root/lianel/dc /root/hosting-base/lianel/dc; do
    if [ -f "$d/docker-compose.yaml" ] || [ -f "$d/docker-compose.backend.yaml" ]; then
      DC_DIR="$d"
      break
    fi
  done
fi
if [ -z "$DC_DIR" ] || [ ! -d "$DC_DIR" ]; then
  echo "❌ Error: Could not find compose directory. Set DC_DIR."
  exit 1
fi
echo "Using directory: $DC_DIR"
cd "$DC_DIR"

if [ -n "${GITHUB_TOKEN:-}" ] && [ -n "${GITHUB_ACTOR:-}" ]; then
  echo "$GITHUB_TOKEN" | docker login ghcr.io -u "$GITHUB_ACTOR" --password-stdin 2>/dev/null || true
fi

echo "Pulling $IMAGE_TAG..."
docker pull "$IMAGE_TAG" || { echo "❌ Pull failed"; exit 1; }

docker rmi "$LOCAL_TAG" 2>/dev/null || true
docker tag "$IMAGE_TAG" "$LOCAL_TAG" || { echo "❌ Tag failed"; exit 1; }

COMPOSE_FILES=""
if [ -f "docker-compose.yaml" ] && docker compose -f docker-compose.yaml config --services 2>/dev/null | grep -q "^${SERVICE_NAME}$"; then
  COMPOSE_FILES="-f docker-compose.yaml"
elif [ -f "docker-compose.backend.yaml" ] && docker compose -f docker-compose.backend.yaml config --services 2>/dev/null | grep -q "^${SERVICE_NAME}$"; then
  COMPOSE_FILES="-f docker-compose.backend.yaml"
fi
if [ -z "$COMPOSE_FILES" ]; then
  echo "❌ Error: profile-service not found in compose"
  exit 1
fi

echo "Starting $SERVICE_NAME..."
docker compose $COMPOSE_FILES up -d --force-recreate --no-deps $SERVICE_NAME
sleep 2
docker compose $COMPOSE_FILES ps $SERVICE_NAME
echo "✅ Profile service deployed"
