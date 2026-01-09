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

# Remove the image by tag (but keep it if it's the same as what we'll pull)
# This helps when the same tag is reused (like :latest)
REPO_NAME=$(echo "$IMAGE_TAG" | cut -d':' -f1)
echo "Clearing any cached images from repository: $REPO_NAME"
# Remove images with the same repository but different tags
docker images "$REPO_NAME" --format "{{.Repository}}:{{.Tag}}" 2>/dev/null | while read -r img; do
  if [ "$img" != "$IMAGE_TAG" ] && [ "$img" != "$LOCAL_TAG" ]; then
    echo "Removing old image: $img"
    docker rmi "$img" 2>/dev/null || true
  fi
done

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
  # Use --no-cache to ensure we get the latest image, not a cached version
  if docker pull --no-cache "$IMAGE_TAG" 2>&1; then
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

# Verify the pulled image exists
if ! docker images "$IMAGE_TAG" --format "{{.Repository}}:{{.Tag}}" | grep -q "^${IMAGE_TAG}$"; then
  echo "❌ Error: Image $IMAGE_TAG not found after pull"
  echo "Available images:"
  docker images | grep -E "$(echo $IMAGE_TAG | cut -d':' -f1)" | head -5
  exit 1
fi

# Get the image digest to verify we have the latest
IMAGE_DIGEST=$(docker inspect "$IMAGE_TAG" --format '{{index .RepoDigests 0}}' 2>/dev/null || echo "")
if [ -n "$IMAGE_DIGEST" ]; then
  echo "✅ Image digest: $IMAGE_DIGEST"
fi

# Remove old local tag if it exists (to force update)
echo "Removing old local tag to ensure fresh image..."
docker rmi "$LOCAL_TAG" 2>/dev/null || true

# Tag for local use
echo "Tagging image for local use..."
if ! docker tag "$IMAGE_TAG" "$LOCAL_TAG" 2>&1; then
  echo "❌ Error: Failed to tag image"
  exit 1
fi

# Verify the tag was created
if ! docker images "$LOCAL_TAG" --format "{{.Repository}}:{{.Tag}}" | grep -q "^${LOCAL_TAG}$"; then
  echo "❌ Error: Local tag $LOCAL_TAG not found after tagging"
  exit 1
fi
echo "✅ Image tagged as $LOCAL_TAG"

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

# Stop and remove existing container first to ensure clean state
echo "Stopping and removing existing container..."
docker stop lianel-$SERVICE_NAME 2>/dev/null || true
docker rm lianel-$SERVICE_NAME 2>/dev/null || true

# Try docker compose first, fallback to docker-compose
# We already pulled and tagged the image above, so just recreate the container
# Use --no-deps to avoid recreating dependencies (keycloak, nginx, etc.)
# Use --pull never since we already pulled and tagged the image
echo "Starting container with docker compose..."
if command -v docker >/dev/null 2>&1 && docker compose version >/dev/null 2>&1; then
  # First, verify the service exists in docker-compose.yaml
  if ! docker compose -f docker-compose.yaml config --services | grep -q "^${SERVICE_NAME}$"; then
    echo "⚠️  Warning: Service '$SERVICE_NAME' not found in docker-compose.yaml"
    echo "Available services:"
    docker compose -f docker-compose.yaml config --services
    echo "Falling back to direct container start..."
    docker run -d \
      --name lianel-$SERVICE_NAME \
      --network lianel-network \
      --restart unless-stopped \
      $LOCAL_TAG || {
      echo "❌ Error: Direct container start failed"
      exit 1
    }
  else
    # Use --pull never to use the image we just pulled and tagged
    if docker compose -f docker-compose.yaml up -d --force-recreate --no-deps --pull never $SERVICE_NAME 2>&1; then
      echo "✅ Container started successfully with docker compose"
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
  fi
elif command -v docker-compose >/dev/null 2>&1; then
  if docker-compose -f docker-compose.yaml up -d --force-recreate --no-deps $SERVICE_NAME 2>&1; then
    echo "✅ Container started successfully with docker-compose"
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
