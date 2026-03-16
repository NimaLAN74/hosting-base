#!/bin/bash
# Update IBKR OAuth access token and secret on the remote host .env and restart stock-service.
# Run from repo root. Uses SSH; does not commit or store tokens in the repo.
#
# Usage:
#   IBKR_OAUTH_ACCESS_TOKEN=xxx IBKR_OAUTH_ACCESS_TOKEN_SECRET=yyy REMOTE_HOST=host REMOTE_USER=root \
#     bash lianel/dc/scripts/deployment/update-ibkr-tokens-remote-env.sh
# Or set REMOTE_* and IBKR_* vars in .env and: source .env; bash lianel/dc/scripts/deployment/update-ibkr-tokens-remote-env.sh
#
# Requires: SSH key that can log in as REMOTE_USER@REMOTE_HOST (e.g. -i ~/.ssh/id_ed25519_host).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
REMOTE_HOST="${REMOTE_HOST:-}"
REMOTE_USER="${REMOTE_USER:-root}"
REMOTE_PORT="${REMOTE_PORT:-22}"
SSH_KEY="${SSH_KEY:-$HOME/.ssh/id_ed25519_host}"

if [ -f "$REPO_ROOT/.env" ]; then
  set -a
  source "$REPO_ROOT/.env" 2>/dev/null || true
  set +a
fi
[ -n "${REMOTE_HOST:-}" ] || { echo "REMOTE_HOST required. Set in env or .env"; exit 1; }

IBKR_OAUTH_ACCESS_TOKEN="${IBKR_OAUTH_ACCESS_TOKEN:-}"
IBKR_OAUTH_ACCESS_TOKEN_SECRET="${IBKR_OAUTH_ACCESS_TOKEN_SECRET:-}"
if [ -z "$IBKR_OAUTH_ACCESS_TOKEN" ] || [ -z "$IBKR_OAUTH_ACCESS_TOKEN_SECRET" ]; then
  echo "Set both IBKR_OAUTH_ACCESS_TOKEN and IBKR_OAUTH_ACCESS_TOKEN_SECRET (from IBKR OAuth portal after clicking Generate Token)"
  exit 1
fi

# Escape single quotes for safe use inside single-quoted remote string: ' -> '\''
escape_sq() { echo "$1" | sed "s/'/'\\\\''/g"; }
TOKEN_ESC=$(escape_sq "$IBKR_OAUTH_ACCESS_TOKEN")
SECRET_ESC=$(escape_sq "$IBKR_OAUTH_ACCESS_TOKEN_SECRET")

REMOTE_SCRIPT='
set -euo pipefail
DC_DIR=""
for d in /root/lianel/dc /root/hosting-base/lianel/dc; do
  [ -f "$d/docker-compose.infra.yaml" ] && DC_DIR="$d" && break
done
[ -n "$DC_DIR" ] || { echo "No compose dir found on remote"; exit 1; }
ENV_FILE="$DC_DIR/.env"
touch "$ENV_FILE"
for var in IBKR_OAUTH_ACCESS_TOKEN IBKR_OAUTH_ACCESS_TOKEN_SECRET; do
  eval "val=\${${var}:-}"
  if [ -n "$val" ]; then
    tmp=$(mktemp)
    grep -v "^${var}=" "$ENV_FILE" > "$tmp" || true
    printf "%s=%s\n" "$var" "$val" >> "$tmp"
    mv "$tmp" "$ENV_FILE"
    echo "Configured $var in $ENV_FILE"
  fi
done
echo "Restarting stock-service to pick up new tokens..."
cd "$DC_DIR"
docker compose -f docker-compose.infra.yaml -f docker-compose.stock-service.yaml up -d --force-recreate --no-deps stock-service 2>/dev/null || true
echo "Done."
'

SSH_OPTS=(-o ConnectTimeout=30 -o StrictHostKeyChecking=accept-new -p "${REMOTE_PORT}")
[ -f "${SSH_KEY}" ] && SSH_OPTS+=(-i "$SSH_KEY")
SSH_CMD="ssh ${SSH_OPTS[*]} ${REMOTE_USER}@${REMOTE_HOST}"
$SSH_CMD "IBKR_OAUTH_ACCESS_TOKEN='${TOKEN_ESC}' IBKR_OAUTH_ACCESS_TOKEN_SECRET='${SECRET_ESC}' bash -s" <<< "$REMOTE_SCRIPT"
echo "✅ IBKR tokens updated on remote .env and stock-service restarted."
