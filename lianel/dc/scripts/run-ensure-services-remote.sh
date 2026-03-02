#!/bin/bash
# Run "ensure all services up" on the remote server via SSH.
# From repo root (or lianel/dc): REMOTE_HOST=your-server SSH_KEY=~/.ssh/deploy_key bash lianel/dc/scripts/run-ensure-services-remote.sh
# Or set REMOTE_HOST and optionally REMOTE_USER, REMOTE_PORT, SSH_KEY in .env at repo root.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
MAINT_DIR="$(cd "$SCRIPT_DIR/maintenance" && pwd)"

REMOTE_HOST="${REMOTE_HOST:-}"
REMOTE_USER="${REMOTE_USER:-root}"
REMOTE_PORT="${REMOTE_PORT:-22}"
SSH_KEY="${SSH_KEY:-}"

if [ -f "$REPO_ROOT/.env" ]; then
  set -a
  source "$REPO_ROOT/.env" 2>/dev/null || true
  set +a
  REMOTE_HOST="${REMOTE_HOST:-}"
  REMOTE_USER="${REMOTE_USER:-root}"
  REMOTE_PORT="${REMOTE_PORT:-22}"
  [ -n "${SSH_KEY:-}" ] || SSH_KEY="${SSH_KEY:-}"
fi

if [ -z "$REMOTE_HOST" ]; then
  echo "Usage: REMOTE_HOST=host [REMOTE_USER=root] [SSH_KEY=~/.ssh/your_key] [REMOTE_PORT=22] $0"
  echo "  Or set REMOTE_HOST (and optionally SSH_KEY) in .env at repo root."
  exit 1
fi

ENSURE_SCRIPT="$MAINT_DIR/ensure-all-services-up-on-server.sh"
if [ ! -f "$ENSURE_SCRIPT" ]; then
  echo "Error: $ENSURE_SCRIPT not found."
  exit 1
fi

SSH_OPTS="-o ConnectTimeout=60 -o StrictHostKeyChecking=accept-new -p ${REMOTE_PORT}"
if [ -n "$SSH_KEY" ] && [ -f "$SSH_KEY" ]; then
  SSH_OPTS="$SSH_OPTS -i $SSH_KEY"
  echo "Using SSH key: $SSH_KEY"
fi

echo "Ensuring all services (stock, comp-ai, monitoring, airflow, infra) on ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_PORT}..."
echo ""

ssh $SSH_OPTS "${REMOTE_USER}@${REMOTE_HOST}" "bash -s" < "$ENSURE_SCRIPT"

echo ""
echo "Done."
