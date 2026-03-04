#!/usr/bin/env bash
# Call the quotes endpoint FROM the server using pairs from the data source (watchlist_items).
# Backend loads (symbol, provider) from DB and fetches prices for those pairs — no hardcoded pairs.
# Usage: REMOTE_HOST=72.60.80.84 REMOTE_USER=root SSH_KEY=~/.ssh/id_ed25519_host ./verify-quotes-from-server.sh
set -e
REMOTE_HOST="${REMOTE_HOST:-72.60.80.84}"
REMOTE_USER="${REMOTE_USER:-root}"
SSH_KEY="${SSH_KEY:-$HOME/.ssh/id_ed25519_host}"
CONTAINER="${CONTAINER:-lianel-stock-monitoring-service}"
BACKEND_PORT="${BACKEND_PORT:-3003}"

echo "=== Quotes from server (pairs from watchlist_items in DB) ==="
ssh -F /dev/null -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o ConnectTimeout=15 -i "${SSH_KEY}" "${REMOTE_USER}@${REMOTE_HOST}" \
  "docker exec ${CONTAINER} curl -sS -i -H 'Accept: application/json' 'http://localhost:${BACKEND_PORT}/internal/quotes'" 2>&1
