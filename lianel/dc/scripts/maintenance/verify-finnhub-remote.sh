#!/bin/bash
# Verify Finnhub API key on the remote host: call Finnhub quote API and report result.
# Run from repo root. Uses SSH (see lianel/dc/scripts/SSH-CONFIG.md).
#   bash lianel/dc/scripts/maintenance/verify-finnhub-remote.sh
# Or run ON the remote: get key from container and curl Finnhub.

set -euo pipefail

REMOTE_HOST="${REMOTE_HOST:-72.60.80.84}"
REMOTE_USER="${REMOTE_USER:-root}"
SSH_KEY="${SSH_KEY:-$HOME/.ssh/id_ed25519_host}"

REMOTE_SCRIPT='
KEY=$(docker exec lianel-stock-monitoring-service env 2>/dev/null | grep "^FINNHUB_API_KEY=" | cut -d= -f2-);
if [ -z "$KEY" ]; then
  echo "FINNHUB_API_KEY not set in container.";
  exit 1;
fi;
echo "Calling Finnhub quote API for AAPL...";
RESP=$(curl -sS "https://finnhub.io/api/v1/quote?symbol=AAPL&token=${KEY}");
if echo "$RESP" | grep -q "\"error\""; then
  echo "Finnhub error: $RESP";
  exit 1;
fi;
C=$(echo "$RESP" | grep -o "\"c\":[0-9.]*" | cut -d: -f2);
if [ -n "$C" ] && [ "$C" != "0" ]; then
  echo "OK: Finnhub returned price c=$C for AAPL";
  exit 0;
fi;
echo "Unexpected response: $RESP";
exit 1;
'

if [ -n "${SSH_KEY}" ] && [ -f "${SSH_KEY}" ]; then
  ssh -F /dev/null -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o IdentitiesOnly=yes -i "${SSH_KEY}" "${REMOTE_USER}@${REMOTE_HOST}" "bash -s" <<< "$REMOTE_SCRIPT"
else
  eval "$REMOTE_SCRIPT"
fi
