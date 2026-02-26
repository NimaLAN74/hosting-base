#!/bin/bash
# Add FINNHUB_API_KEY and FINNHUB_WEBHOOK_SECRET to the remote host .env (used by stock-monitoring).
# Run from repo root. Uses SSH; does not commit or store keys in the repo.
#
# Usage:
#   FINNHUB_API_KEY=xxx FINNHUB_WEBHOOK_SECRET=yyy REMOTE_HOST=host REMOTE_USER=root \
#     bash lianel/dc/scripts/deployment/add-finnhub-keys-remote-env.sh
# Or set REMOTE_* and Finnhub vars in .env and: source .env; bash lianel/dc/scripts/deployment/add-finnhub-keys-remote-env.sh
#
# Requires: SSH key that can log in as REMOTE_USER@REMOTE_HOST (e.g. -i ~/.ssh/deploy_key).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
REMOTE_HOST="${REMOTE_HOST:-}"
REMOTE_USER="${REMOTE_USER:-root}"
REMOTE_PORT="${REMOTE_PORT:-22}"

if [ -f "$REPO_ROOT/.env" ]; then
  set -a
  source "$REPO_ROOT/.env" 2>/dev/null || true
  set +a
fi
[ -n "${REMOTE_HOST:-}" ] || { echo "REMOTE_HOST required. Set in env or .env"; exit 1; }

FINNHUB_API_KEY="${FINNHUB_API_KEY:-}"
FINNHUB_WEBHOOK_SECRET="${FINNHUB_WEBHOOK_SECRET:-}"
if [ -z "$FINNHUB_API_KEY" ] && [ -z "$FINNHUB_WEBHOOK_SECRET" ]; then
  echo "Set at least one of FINNHUB_API_KEY or FINNHUB_WEBHOOK_SECRET (in env or .env)"
  exit 1
fi

# Build the inline script that runs on the remote: find DC_DIR and update .env
REMOTE_SCRIPT='
set -euo pipefail
DC_DIR=""
for d in /root/lianel/dc /root/hosting-base/lianel/dc; do
  [ -f "$d/docker-compose.infra.yaml" ] && DC_DIR="$d" && break
done
[ -n "$DC_DIR" ] || { echo "No compose dir found on remote"; exit 1; }
ENV_FILE="$DC_DIR/.env"
touch "$ENV_FILE"
for var in FINNHUB_API_KEY FINNHUB_WEBHOOK_SECRET; do
  eval "val=\${${var}:-}"
  if [ -n "$val" ]; then
    tmp=$(mktemp)
    grep -v "^${var}=" "$ENV_FILE" > "$tmp" || true
    printf "%s=%s\n" "$var" "$val" >> "$tmp"
    mv "$tmp" "$ENV_FILE"
    echo "Configured $var in $ENV_FILE"
  fi
done
echo "Restarting stock-monitoring-service to pick up env..."
cd "$DC_DIR"
docker compose -f docker-compose.infra.yaml -f docker-compose.stock-monitoring.yaml up -d --force-recreate --no-deps stock-monitoring-service 2>/dev/null || true
echo "Done."
'

# Pass the two vars to the remote (safe for values without single quotes)
SSH_CMD="ssh -o ConnectTimeout=30 -o StrictHostKeyChecking=accept-new -p ${REMOTE_PORT} ${REMOTE_USER}@${REMOTE_HOST}"
$SSH_CMD "FINNHUB_API_KEY='${FINNHUB_API_KEY}' FINNHUB_WEBHOOK_SECRET='${FINNHUB_WEBHOOK_SECRET}' bash -s" <<< "$REMOTE_SCRIPT"
echo "✅ Finnhub keys added to remote .env (and service restarted if compose available)"