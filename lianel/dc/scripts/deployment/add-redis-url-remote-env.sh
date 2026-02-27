#!/bin/bash
# Add REDIS_URL to the remote host .env (for stock-monitoring real-time intraday chart).
# Run from repo root. Uses SSH.
#
# Usage:
#   REMOTE_HOST=host REMOTE_USER=root bash lianel/dc/scripts/deployment/add-redis-url-remote-env.sh
# Or set REMOTE_HOST in .env and: source lianel/dc/.env; bash lianel/dc/scripts/deployment/add-redis-url-remote-env.sh
#
# Requires: SSH key that can log in as REMOTE_USER@REMOTE_HOST.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
REMOTE_HOST="${REMOTE_HOST:-72.60.80.84}"
REMOTE_USER="${REMOTE_USER:-root}"
REMOTE_PORT="${REMOTE_PORT:-22}"
SSH_KEY="${SSH_KEY:-$HOME/.ssh/id_ed25519_host}"

# Literal line for remote .env (${REDIS_PASSWORD} expands when compose reads .env)
REMOTE_SCRIPT='
set -eo pipefail
DC_DIR=""
for d in /root/lianel/dc /root/hosting-base/lianel/dc; do
  [ -f "$d/docker-compose.infra.yaml" ] && DC_DIR="$d" && break
done
[ -n "$DC_DIR" ] || { echo "No compose dir found on remote"; exit 1; }
ENV_FILE="$DC_DIR/.env"
touch "$ENV_FILE"
grep -v "^REDIS_URL=" "$ENV_FILE" > "${ENV_FILE}.tmp" || true
printf "%s\n" '\''REDIS_URL=redis://:${REDIS_PASSWORD}@dc-redis-1:6379/1'\'' >> "${ENV_FILE}.tmp"
mv "${ENV_FILE}.tmp" "$ENV_FILE"
echo "Configured REDIS_URL in $ENV_FILE"
cd "$DC_DIR"
docker compose -f docker-compose.infra.yaml -f docker-compose.stock-monitoring.yaml up -d --force-recreate --no-deps stock-monitoring-service 2>/dev/null || true
echo "Done."
'

SSH_OPTS="-o ConnectTimeout=30 -o StrictHostKeyChecking=accept-new -p ${REMOTE_PORT}"
[ -f "$SSH_KEY" ] && SSH_OPTS="$SSH_OPTS -i $SSH_KEY"
ssh $SSH_OPTS "${REMOTE_USER}@${REMOTE_HOST}" "bash -s" <<< "$REMOTE_SCRIPT"
echo "✅ REDIS_URL added to remote .env (and stock-monitoring-service restarted if compose available)"
