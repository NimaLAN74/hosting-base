#!/bin/bash
# Run fix-502-keycloak-on-server.sh on the remote host via SSH.
# Use when Sync Nginx (Fix Login) workflow fails with "Preflight SSH timeout" (server unreachable from GitHub).
#
# From repo root:
#   REMOTE_HOST=your-server REMOTE_USER=root bash lianel/dc/scripts/run-fix-502-via-ssh.sh
# Or set in .env and: source .env; bash lianel/dc/scripts/run-fix-502-via-ssh.sh
#
# Requires: SSH key that can log in as REMOTE_USER@REMOTE_HOST (e.g. ssh -i ~/.ssh/deploy_key ...).

set -e

# Script lives in hosting-base/lianel/dc/scripts/; repo root = hosting-base
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
REMOTE_HOST="${REMOTE_HOST:-}"
REMOTE_USER="${REMOTE_USER:-root}"
REMOTE_PORT="${REMOTE_PORT:-22}"

if [ -z "$REMOTE_HOST" ]; then
  if [ -f "$REPO_ROOT/.env" ]; then
    source "$REPO_ROOT/.env" 2>/dev/null || true
  fi
  REMOTE_HOST="${REMOTE_HOST:-}"
fi

if [ -z "$REMOTE_HOST" ]; then
  echo "Usage: REMOTE_HOST=host REMOTE_USER=root [REMOTE_PORT=22] $0"
  echo "  Or set REMOTE_HOST (and optionally REMOTE_USER) in .env"
  exit 1
fi

FIX_SCRIPT="$SCRIPT_DIR/fix-502-keycloak-on-server.sh"

echo "Running fix-502 on ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_PORT}..."
ssh -o ConnectTimeout=30 -o StrictHostKeyChecking=accept-new -p "${REMOTE_PORT}" "${REMOTE_USER}@${REMOTE_HOST}" "bash -s" < "$FIX_SCRIPT"
echo "✅ Done. Test: https://www.lianel.se/auth/ (expect 302 or login page, not 502)"
