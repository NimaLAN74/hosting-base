#!/bin/bash
# Deploy profile service. Copied to remote by deploy-profile-service.yml.
# Usage: ./deploy-profile-service.sh <IMAGE_TAG>
# On server set DC_DIR if compose is not in /root/lianel/dc or /root/hosting-base/lianel/dc.
set -euo pipefail

IMAGE_TAG="${1:-}"
SERVICE_NAME="profile-service"
LOCAL_TAG="lianel-profile-service:latest"
CONTAINER_NAME="lianel-profile-service"

if [ -z "$IMAGE_TAG" ]; then
  echo "❌ Error: IMAGE_TAG is required"
  echo "Usage: $0 <IMAGE_TAG>"
  exit 1
fi

echo "=== Deploying Profile Service ==="
echo "Image: $IMAGE_TAG"

DC_DIR="${DC_DIR:-}"
if [ -z "$DC_DIR" ]; then
  for d in /root/lianel/dc /root/hosting-base/lianel/dc /root/dc .; do
    [ -z "$d" ] && continue
    if [ -f "$d/docker-compose.yaml" ] || [ -f "$d/docker-compose.backend.yaml" ]; then
      DC_DIR="$d"
      break
    fi
  done
fi
if [ -z "$DC_DIR" ] || [ ! -d "$DC_DIR" ]; then
  echo "❌ Error: Could not find compose directory (tried /root/lianel/dc, /root/hosting-base/lianel/dc, /root/dc, .). Set DC_DIR."
  exit 1
fi
echo "Using directory: $DC_DIR"
cd "$DC_DIR"

if [ -n "${GITHUB_TOKEN:-}" ] && [ -n "${GITHUB_ACTOR:-}" ]; then
  echo "Logging in to ghcr.io..."
  if ! echo "$GITHUB_TOKEN" | docker login ghcr.io -u "$GITHUB_ACTOR" --password-stdin; then
    echo "❌ ghcr.io login failed (check GITHUB_TOKEN / GITHUB_ACTOR and registry access)"
    exit 1
  fi
fi

echo "Pulling $IMAGE_TAG..."
if ! docker pull "$IMAGE_TAG"; then
  echo "❌ Pull failed. Ensure the image exists and ghcr.io is logged in (GITHUB_TOKEN/GITHUB_ACTOR)."
  exit 1
fi

docker rmi "$LOCAL_TAG" 2>/dev/null || true
docker tag "$IMAGE_TAG" "$LOCAL_TAG" || { echo "❌ Tag failed"; exit 1; }

COMPOSE_FILES=""
if [ -f "docker-compose.yaml" ] && docker compose -f docker-compose.yaml config --services 2>/dev/null | grep -q "^${SERVICE_NAME}$"; then
  COMPOSE_FILES="-f docker-compose.yaml"
elif [ -f "docker-compose.infra.yaml" ] && [ -f "docker-compose.backend.yaml" ] && docker compose -f docker-compose.infra.yaml -f docker-compose.backend.yaml config --services 2>/dev/null | grep -q "^${SERVICE_NAME}$"; then
  COMPOSE_FILES="-f docker-compose.infra.yaml -f docker-compose.backend.yaml"
elif [ -f "docker-compose.backend.yaml" ] && docker compose -f docker-compose.backend.yaml config --services 2>/dev/null | grep -q "^${SERVICE_NAME}$"; then
  COMPOSE_FILES="-f docker-compose.backend.yaml"
fi

if [ -n "$COMPOSE_FILES" ]; then
  echo "Starting $SERVICE_NAME with: docker compose $COMPOSE_FILES up -d ..."
  if docker compose $COMPOSE_FILES up -d --force-recreate --no-deps $SERVICE_NAME; then
    sleep 2
    docker compose $COMPOSE_FILES ps $SERVICE_NAME
    echo "✅ Profile service deployed"
    exit 0
  fi
  echo "⚠️ docker compose up failed, trying docker run fallback..."
fi

# Fallback: start container directly so deploy succeeds even if compose layout differs
echo "Starting $CONTAINER_NAME with docker run..."
docker stop "$CONTAINER_NAME" 2>/dev/null || true
docker rm "$CONTAINER_NAME" 2>/dev/null || true
ENV_FILE_ARGS=""
[ -f ".env" ] && ENV_FILE_ARGS="--env-file .env"
if docker network inspect lianel-network &>/dev/null; then
  docker run -d \
    --name "$CONTAINER_NAME" \
    --network lianel-network \
    --restart unless-stopped \
    -e PORT=3000 \
    -e RUST_LOG=info \
    $ENV_FILE_ARGS \
    "$LOCAL_TAG"
else
  docker run -d \
    --name "$CONTAINER_NAME" \
    --restart unless-stopped \
    -e PORT=3000 \
    -e RUST_LOG=info \
    -p 3000:3000 \
    $ENV_FILE_ARGS \
    "$LOCAL_TAG"
fi
echo "✅ Profile service deployed (docker run)"
