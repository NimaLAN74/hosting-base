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

# Also remove any images with the same repository to clear cache
# This helps when the same tag is reused (like :latest)
REPO_NAME=$(echo "$IMAGE_TAG" | cut -d':' -f1)
echo "Removing any cached images from repository: $REPO_NAME"
# Use a safer approach that works on all systems (xargs -r is GNU-specific)
IMAGE_IDS=$(docker images "$REPO_NAME" --format "{{.ID}}" 2>/dev/null || true)
if [ -n "$IMAGE_IDS" ]; then
  echo "$IMAGE_IDS" | while read -r img_id; do
    [ -n "$img_id" ] && docker rmi -f "$img_id" 2>/dev/null || true
  done
fi

# Clear any stale Docker auth
docker logout ghcr.io 2>/dev/null || true

# Authenticate if credentials provided
if [ -n "${GITHUB_TOKEN:-}" ] && [ -n "${GITHUB_ACTOR:-}" ]; then
  echo "Authenticating with GitHub..."
  echo "$GITHUB_TOKEN" | docker login ghcr.io -u "$GITHUB_ACTOR" --password-stdin 2>&1 || true
fi

# Force pull the latest image (always pull, even if local copy exists)
echo "Pulling latest image (forcing update)..."
PULL_ATTEMPTS=3
PULL_SUCCESS=false

for attempt in $(seq 1 $PULL_ATTEMPTS); do
  echo "Pull attempt $attempt of $PULL_ATTEMPTS..."
  if docker pull "$IMAGE_TAG" 2>&1; then
    echo "✅ Image pulled successfully on attempt $attempt"
    PULL_SUCCESS=true
    break
  else
    PULL_EXIT=$?
    echo "⚠️  Pull attempt $attempt failed (exit code: $PULL_EXIT)"
    if [ $attempt -lt $PULL_ATTEMPTS ]; then
      echo "Waiting 2 seconds before retry..."
      sleep 2
    fi
  fi
done

if [ "$PULL_SUCCESS" = false ]; then
  echo "❌ Error: Failed to pull image after $PULL_ATTEMPTS attempts"
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

# Verify the image exists before trying to start
echo "Verifying image exists..."
if ! docker images "$LOCAL_TAG" --format "{{.Repository}}:{{.Tag}}" | grep -q "^${LOCAL_TAG}$"; then
  echo "❌ Error: Local image tag $LOCAL_TAG not found after pull and tag"
  echo "Available images:"
  docker images | grep -E "lianel-frontend|ghcr.io" | head -5
  exit 1
fi

# Try docker compose first, fallback to docker-compose
# We already pulled and tagged the image above, so just recreate the container
# Use --no-deps to avoid recreating dependencies (keycloak, nginx, etc.)
echo "Starting container with docker compose..."
if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
  if docker compose -f docker-compose.yaml up -d --force-recreate --no-deps $SERVICE_NAME 2>&1; then
    echo "✅ Container started successfully"
  else
    COMPOSE_EXIT=$?
    echo "❌ Error: docker compose failed (exit code: $COMPOSE_EXIT)"
    echo "Trying alternative approach..."
    # Fallback: start container directly
    docker run -d \
      --name lianel-$SERVICE_NAME \
      --network lianel-network \
      --restart unless-stopped \
      $LOCAL_TAG || {
      echo "❌ Error: Direct container start also failed"
      exit 1
    }
  fi
elif command -v docker-compose >/dev/null 2>&1; then
  if docker-compose -f docker-compose.yaml up -d --force-recreate --no-deps $SERVICE_NAME 2>&1; then
    echo "✅ Container started successfully"
  else
    echo "❌ Error: docker-compose failed"
    exit 1
  fi
else
  echo "❌ Error: Neither 'docker compose' nor 'docker-compose' found"
  exit 1
fi

# Verify deployment
echo "Waiting for container to start..."
sleep 8  # Increased wait time for container to fully start

# Check if container is running
if docker ps --format '{{.Names}}' | grep -q "^lianel-$SERVICE_NAME$"; then
  echo "✅ Container is running"
  docker ps --filter "name=lianel-$SERVICE_NAME" --format "table {{.Names}}\t{{.Status}}\t{{.Image}}\t{{.CreatedAt}}"
  echo ""
  
  # Verify the container is using the correct image
  CONTAINER_IMAGE=$(docker inspect lianel-$SERVICE_NAME --format '{{.Config.Image}}' 2>/dev/null || echo "unknown")
  echo "Container image: $CONTAINER_IMAGE"
  echo "Expected image: $LOCAL_TAG"
  
  if [ "$CONTAINER_IMAGE" != "$LOCAL_TAG" ]; then
    echo "⚠️  Warning: Container image doesn't match expected tag"
    echo "This might indicate the image wasn't updated correctly"
  fi
  
  echo ""
  echo "Image details:"
  docker inspect "$LOCAL_TAG" --format 'Repository: {{.RepoDigests}}' 2>/dev/null || \
  docker inspect "$IMAGE_TAG" --format 'Digest: {{index .RepoDigests 0}}' 2>/dev/null || \
  echo "Digest not available"
  
  echo ""
  echo "✅ Deployment successful"
else
  echo "❌ Container not running after startup"
  echo "Checking stopped containers:"
  docker ps -a --filter "name=lianel-$SERVICE_NAME" --format "table {{.Names}}\t{{.Status}}\t{{.Image}}\t{{.CreatedAt}}" || true
  echo ""
  echo "Recent logs:"
  docker logs lianel-$SERVICE_NAME --tail 30 2>&1 || true
  echo ""
  echo "Container exit code:"
  docker inspect lianel-$SERVICE_NAME --format '{{.State.ExitCode}}' 2>/dev/null || echo "Could not get exit code"
  exit 1
fi

# Cleanup old images
echo "Cleaning up old images..."
docker image prune -f

echo "=== Deployment Complete ==="
# Test
