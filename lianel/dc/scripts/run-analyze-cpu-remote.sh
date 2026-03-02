#!/bin/bash
# Run CPU analysis on the remote server via SSH (with key).
# From repo root:
#   REMOTE_HOST=your-server SSH_KEY=~/.ssh/deploy_key bash lianel/dc/scripts/run-analyze-cpu-remote.sh
#   REMOTE_HOST=your-server SSH_KEY=~/.ssh/deploy_key bash lianel/dc/scripts/run-analyze-cpu-remote.sh fix
# Or set REMOTE_HOST (and optionally REMOTE_USER, REMOTE_PORT, SSH_KEY) in .env.
#
# First arg "fix" or FIX=1: after analysis, restart top CPU containers on the server.

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
MAINT_DIR="$(cd "$SCRIPT_DIR/maintenance" && pwd)"

REMOTE_HOST="${REMOTE_HOST:-}"
REMOTE_USER="${REMOTE_USER:-root}"
REMOTE_PORT="${REMOTE_PORT:-22}"
SSH_KEY="${SSH_KEY:-}"
FIX="${FIX:-0}"

if [ "${1:-}" = "fix" ]; then
  FIX=1
fi

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
  echo "Usage: REMOTE_HOST=host [REMOTE_USER=root] [SSH_KEY=~/.ssh/your_key] [REMOTE_PORT=22] $0 [fix]"
  echo "  Or set REMOTE_HOST (and optionally SSH_KEY) in .env at repo root."
  echo "  'fix' or FIX=1: restart top CPU containers after analysis."
  exit 1
fi

ANALYZE_SCRIPT="$MAINT_DIR/analyze-cpu-on-server.sh"
if [ ! -f "$ANALYZE_SCRIPT" ]; then
  echo "Error: $ANALYZE_SCRIPT not found."
  exit 1
fi

SSH_OPTS="-o ConnectTimeout=30 -o StrictHostKeyChecking=accept-new -p ${REMOTE_PORT}"
if [ -n "$SSH_KEY" ] && [ -f "$SSH_KEY" ]; then
  SSH_OPTS="$SSH_OPTS -i $SSH_KEY"
  echo "Using SSH key: $SSH_KEY"
fi

echo "Running CPU analysis on ${REMOTE_USER}@${REMOTE_HOST}:${REMOTE_PORT}..."
echo ""

# Run analysis script on remote; FIX=1 restarts top CPU containers
ssh $SSH_OPTS "${REMOTE_USER}@${REMOTE_HOST}" "FIX=$FIX bash -s" < "$ANALYZE_SCRIPT"

echo ""
echo "Done. If CPU is still high, run with 'fix' to restart top containers: $0 fix"
