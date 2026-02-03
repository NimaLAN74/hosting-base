#!/bin/bash
# Deploy webserver_config.py to remote host and restart Airflow apiserver.
# Prefer pipeline: push to config/webserver_config.py (or workflow) triggers
#   .github/workflows/deploy-airflow-webserver-config.yml (no local SSH needed).
# For manual deploy from repo root:
#   bash lianel/dc/scripts/deployment/deploy-airflow-webserver-config.sh
# Uses SSH from scripts/SSH-CONFIG.md (host 72.60.80.84, key ~/.ssh/id_ed25519_host).

set -e
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/../.." && pwd)}"
REMOTE_CONFIG="/root/lianel/dc/config/webserver_config.py"
SSH_OPTS="-F /dev/null -o UserKnownHostsFile=/dev/null -o StrictHostKeyChecking=no -o IdentitiesOnly=yes -i ~/.ssh/id_ed25519_host"

echo "=== 1. Copy webserver_config.py to remote ==="
echo "scp ... $REPO_ROOT/config/webserver_config.py root@72.60.80.84:$REMOTE_CONFIG"
scp $SSH_OPTS "$REPO_ROOT/config/webserver_config.py" root@72.60.80.84:"$REMOTE_CONFIG"

echo ""
echo "=== 2. Restart Airflow apiserver on remote ==="
echo "ssh ... 'cd /root/lianel/dc && docker compose -f docker-compose.airflow.yaml restart airflow-apiserver'"
ssh $SSH_OPTS root@72.60.80.84 "cd /root/lianel/dc && docker compose -f docker-compose.airflow.yaml restart airflow-apiserver"

echo ""
echo "=== Done ==="
