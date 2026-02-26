#!/bin/bash
# Renew Let's Encrypt certificates on the remote host.
# Certbot on the host uses 'standalone' mode, so nginx must be stopped briefly.
#
# Run from repo root. Requires SSH key (see lianel/dc/scripts/SSH-CONFIG.md).
#   REMOTE_HOST=72.60.80.84 REMOTE_USER=root SSH_KEY=~/.ssh/id_ed25519_host \
#     bash lianel/dc/scripts/maintenance/renew-ssl-remote.sh
# Or run ON the remote host: bash renew-ssl-remote.sh  (then set RUN_ON_REMOTE=1 and copy script there)

set -euo pipefail

REMOTE_HOST="${REMOTE_HOST:-72.60.80.84}"
REMOTE_USER="${REMOTE_USER:-root}"
SSH_KEY="${SSH_KEY:-$HOME/.ssh/id_ed25519_host}"
REMOTE_PORT="${REMOTE_PORT:-22}"

SSH_OPTS="-o ConnectTimeout=15 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o IdentitiesOnly=yes"
[ -n "${SSH_KEY}" ] && [ -f "${SSH_KEY}" ] && SSH_OPTS="-i ${SSH_KEY} $SSH_OPTS"

REMOTE_SCRIPT='
set -e
echo "Stopping nginx-proxy..."
docker stop nginx-proxy 2>/dev/null || true
sleep 2
echo "Running certbot renew..."
certbot renew --non-interactive --agree-tos --force-renewal
echo "Starting nginx-proxy..."
docker start nginx-proxy
sleep 2
docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload
echo "✅ SSL renewal done; nginx reloaded."
'

ssh $SSH_OPTS -p "${REMOTE_PORT}" "${REMOTE_USER}@${REMOTE_HOST}" "bash -s" <<< "$REMOTE_SCRIPT"
