#!/bin/bash
# Deploy stock-monitoring frontend (UI) only. Uses same DC_DIR logic as deploy-frontend.
# Usage: ./deploy-stock-monitoring-frontend.sh <IMAGE_TAG>
set -euo pipefail

IMAGE_TAG="${1:-}"
SERVICE_NAME="stock-monitoring-ui"

if [ -z "$IMAGE_TAG" ]; then
  echo "❌ Error: IMAGE_TAG is required"
  echo "Usage: $0 <IMAGE_TAG>"
  exit 1
fi

DC_DIR="${DC_DIR:-}"
if [ -z "$DC_DIR" ]; then
  for d in /root/lianel/dc /root/hosting-base/lianel/dc; do
    if [ -f "$d/docker-compose.infra.yaml" ]; then
      DC_DIR="$d"
      break
    fi
  done
fi
if [ -z "$DC_DIR" ] || [ ! -d "$DC_DIR" ]; then
  echo "❌ Error: Could not find compose directory (tried /root/lianel/dc, /root/hosting-base/lianel/dc). Set DC_DIR."
  exit 1
fi
echo "Using directory: $DC_DIR"
cd "$DC_DIR"

if [ -n "${GITHUB_TOKEN:-}" ] && [ -n "${GITHUB_ACTOR:-}" ]; then
  echo "$GITHUB_TOKEN" | docker login ghcr.io -u "$GITHUB_ACTOR" --password-stdin 2>/dev/null || true
fi

echo "Pulling $IMAGE_TAG..."
docker pull "$IMAGE_TAG" || { echo "❌ Pull failed"; exit 1; }

echo "Starting $SERVICE_NAME..."
docker compose -f docker-compose.infra.yaml -f docker-compose.stock-monitoring.yaml up -d --force-recreate --no-deps $SERVICE_NAME
sleep 3
docker compose -f docker-compose.infra.yaml -f docker-compose.stock-monitoring.yaml ps $SERVICE_NAME
echo "✅ Stock monitoring frontend deployed"
