#!/bin/bash
# Prepare remote server for IBKR authentication: create PEM directory, ensure .env has IBKR vars.
# Usage: REMOTE_HOST=72.60.80.84 REMOTE_USER=root SSH_KEY=~/.ssh/id_ed25519_host ./prepare-ibkr-remote.sh
# Or run from repo root: bash lianel/dc/scripts/deployment/prepare-ibkr-remote.sh
set -euo pipefail

REMOTE_HOST="${REMOTE_HOST:-72.60.80.84}"
REMOTE_USER="${REMOTE_USER:-root}"
REMOTE_PORT="${REMOTE_PORT:-22}"
SSH_KEY="${SSH_KEY:-$HOME/.ssh/id_ed25519_host}"

SSH_OPTS=(-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=30 -i "$SSH_KEY" -p "$REMOTE_PORT")

echo "Preparing IBKR authentication on ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_PORT}"

# Find DC dir on remote and create stock-service-ibkr next to it
ssh "${SSH_OPTS[@]}" "${REMOTE_USER}@${REMOTE_HOST}" 'set -e
  for d in /root/lianel/dc /root/hosting-base/lianel/dc; do
    if [ -f "$d/docker-compose.infra.yaml" ]; then
      DC_DIR="$d"
      break
    fi
  done
  if [ -z "${DC_DIR:-}" ]; then
    echo "❌ Could not find compose directory (docker-compose.infra.yaml)"
    exit 1
  fi
  echo "Using DC_DIR=$DC_DIR"
  IBKR_DIR="$DC_DIR/stock-service-ibkr"
  mkdir -p "$IBKR_DIR"
  chmod 700 "$IBKR_DIR"
  echo "✅ Created $IBKR_DIR (mode 700)"

  # Ensure .env has IBKR var names (empty) so deploy or manual edit can add values later
  ENV_FILE="$DC_DIR/.env"
  touch "$ENV_FILE"
  for var in IBKR_OAUTH_CONSUMER_KEY IBKR_OAUTH_ACCESS_TOKEN IBKR_OAUTH_ACCESS_TOKEN_SECRET IBKR_OAUTH_REALM; do
    if ! grep -q "^${var}=" "$ENV_FILE" 2>/dev/null; then
      echo "${var}=" >> "$ENV_FILE"
      echo "  Added empty $var"
    fi
  done
  echo "✅ .env ready for IBKR credentials (PEM paths use compose defaults: /app/ibkr-conf/*.pem)"

  # README for admin
  cat > "$IBKR_DIR/README.txt" << "READMEEOF"
IBKR OAuth 1.0a – PEM files for Live Session Token (LST)

Place these files here (from IBKR Self Service Portal):
  - dhparam.pem                  (DH parameter)
  - private_encryption.pem       (encryption key)
  - private_signature.pem       (signature key)

Then set in the same directory .env (or in the parent .env):
  IBKR_OAUTH_CONSUMER_KEY=Your9Char
  IBKR_OAUTH_ACCESS_TOKEN=...
  IBKR_OAUTH_ACCESS_TOKEN_SECRET=...  (base64)
  IBKR_OAUTH_REALM=limited_poa

Restart the stock service after adding credentials:
  docker compose -f docker-compose.infra.yaml -f docker-compose.stock-service.yaml up -d --force-recreate --no-deps stock-service
READMEEOF
  echo "✅ Wrote $IBKR_DIR/README.txt"
  ls -la "$IBKR_DIR"
'

echo "✅ Remote server prepared for IBKR authentication."
echo "Next: 1) Add PEM files from IBKR portal to stock-service-ibkr/ on the server."
echo "      2) Add IBKR_OAUTH_CONSUMER_KEY, IBKR_OAUTH_ACCESS_TOKEN, IBKR_OAUTH_ACCESS_TOKEN_SECRET to the remote .env."
echo "      3) Redeploy stock service (or run the docker compose command in the README)."
