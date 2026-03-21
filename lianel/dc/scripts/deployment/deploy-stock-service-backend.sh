#!/bin/bash
# Deploy stock service backend only. Uses same DC_DIR logic as deploy-frontend.
# Usage: ./deploy-stock-service-backend.sh <IMAGE_TAG>
set -euo pipefail

IMAGE_TAG="${1:-}"
SERVICE_NAME="stock-service"

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

# Write Keycloak and optional IBKR vars to .env in BOTH possible compose dirs (stock service uses env_file: .env)
ENV_VARS="KEYCLOAK_URL KEYCLOAK_REALM KEYCLOAK_ISSUER_ALT IBKR_OAUTH_CONSUMER_KEY IBKR_OAUTH_ACCESS_TOKEN IBKR_OAUTH_ACCESS_TOKEN_SECRET IBKR_OAUTH_DH_PARAM_PATH IBKR_OAUTH_PRIVATE_ENCRYPTION_KEY_PATH IBKR_OAUTH_PRIVATE_SIGNATURE_KEY_PATH"
for env_dir in /root/lianel/dc /root/hosting-base/lianel/dc; do
  [ -d "$env_dir" ] || continue
  ENV_FILE="$env_dir/.env"
  touch "$ENV_FILE"
  for var in $ENV_VARS; do
    eval "val=\${${var}:-}"
    [ -n "$val" ] || continue

    # Do not overwrite existing non-empty values in ENV_FILE.
    # This prevents CI/CD env from accidentally replacing secrets that were manually set on the server.
    existing="$(grep -E "^${var}=" "$ENV_FILE" 2>/dev/null | tail -n 1 | sed -E "s/^${var}=//" || true)"
    if [ -n "$existing" ]; then
      echo "Keeping existing $var in $ENV_FILE (non-empty)"
      continue
    fi

    tmp_env_file="$(mktemp)"
    grep -v "^${var}=" "$ENV_FILE" > "$tmp_env_file" 2>/dev/null || true
    printf '%s=%s\n' "$var" "$val" >> "$tmp_env_file"
    mv "$tmp_env_file" "$ENV_FILE"
    echo "Configured $var in $ENV_FILE"
  done
done

if [ -n "${GITHUB_TOKEN:-}" ] && [ -n "${GITHUB_ACTOR:-}" ]; then
  echo "$GITHUB_TOKEN" | docker login ghcr.io -u "$GITHUB_ACTOR" --password-stdin 2>/dev/null || true
fi

echo "Pulling $IMAGE_TAG..."
docker pull "$IMAGE_TAG" || { echo "❌ Pull failed"; exit 1; }

echo "Starting $SERVICE_NAME..."
docker compose -f docker-compose.infra.yaml -f docker-compose.stock-service.yaml up -d --force-recreate --no-deps $SERVICE_NAME
sleep 3
docker compose -f docker-compose.infra.yaml -f docker-compose.stock-service.yaml ps $SERVICE_NAME
echo "✅ Stock service backend deployed"

# nginx-proxy runs as a separate container and can keep stale upstream IPs after
# stock-service container recreation. Reload nginx-proxy to re-resolve and stop 502s.
echo "Reloading nginx-proxy..."
docker exec nginx-proxy nginx -t 2>/dev/null && \
  docker exec nginx-proxy nginx -s reload 2>/dev/null && \
  echo "✅ nginx-proxy reloaded" || \
  echo "⚠️ nginx-proxy reload skipped (nginx-proxy or config not ready)"
