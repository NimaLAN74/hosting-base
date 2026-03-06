#!/bin/bash
# Verify frontend and stock-service backend on remote server via SSH.
# Usage: REMOTE_HOST=72.60.80.84 REMOTE_USER=root SSH_KEY=~/.ssh/id_ed25519_host bash lianel/dc/scripts/verify-fe-be-remote.sh
# Or from repo root: bash lianel/dc/scripts/verify-fe-be-remote.sh (uses defaults from SSH-CONFIG.md)
set -e

REMOTE_HOST="${REMOTE_HOST:-72.60.80.84}"
REMOTE_USER="${REMOTE_USER:-root}"
REMOTE_PORT="${REMOTE_PORT:-22}"
SSH_KEY="${SSH_KEY:-$HOME/.ssh/id_ed25519_host}"

SSH_OPTS="-F /dev/null -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o IdentitiesOnly=yes -o ConnectTimeout=15"
[ -n "${SSH_KEY}" ] && [ -f "${SSH_KEY}" ] && SSH_OPTS="$SSH_OPTS -i ${SSH_KEY}"

echo "=== Verify FE and BE on remote ($REMOTE_USER@$REMOTE_HOST) ==="
echo ""

# 1) Containers: frontend, nginx, stock-service
echo "1. Containers (frontend, nginx, stock-service):"
ssh $SSH_OPTS -p "$REMOTE_PORT" "${REMOTE_USER}@${REMOTE_HOST}" \
  "docker ps --format '{{.Names}} {{.Status}}' | grep -E 'lianel-frontend|nginx-proxy|nginx|lianel-stock-service' || true"
echo ""

# 2) Stock backend health (inside container)
echo "2. Stock monitoring backend health (localhost:3003):"
ssh $SSH_OPTS -p "$REMOTE_PORT" "${REMOTE_USER}@${REMOTE_HOST}" \
  "docker exec lianel-stock-service curl -fsS http://localhost:3003/health 2>/dev/null || echo 'FAIL: container or curl failed'"
echo ""

# 3) KEYCLOAK env in stock-service container
echo "3. Stock backend KEYCLOAK env:"
ssh $SSH_OPTS -p "$REMOTE_PORT" "${REMOTE_USER}@${REMOTE_HOST}" \
  "docker exec lianel-stock-service env 2>/dev/null | grep KEYCLOAK || echo 'No KEYCLOAK vars or container missing'"
echo ""

# 4) Public health (via curl from host to nginx)
echo "4. Public API health (from remote host to localhost):"
ssh $SSH_OPTS -p "$REMOTE_PORT" "${REMOTE_USER}@${REMOTE_HOST}" \
  "curl -sS -o /dev/null -w '%{http_code}' http://localhost/api/v1/stock-service/health -H 'Host: www.lianel.se' 2>/dev/null || echo 'FAIL'"
echo ""

echo "=== Done ==="
