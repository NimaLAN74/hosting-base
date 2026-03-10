#!/usr/bin/env bash
# Run IBKR live_session_token script until we get HTTP 200 (green).
# Usage:
#   From local: REMOTE_HOST=72.60.80.84 SSH_KEY=~/.ssh/id_ed25519_host bash lianel/dc/scripts/ibkr/run_ibkr_until_green.sh
#   One shot (no retry loop): RUN_ONCE=1 REMOTE_HOST=... bash run_ibkr_until_green.sh
#   On server:  cd /root/lianel/dc && DC_DIR=. RUN_ONCE=1 bash scripts/ibkr/run_ibkr_until_green.sh
set -euo pipefail
RUN_ONCE="${RUN_ONCE:-0}"

REMOTE_HOST="${REMOTE_HOST:-}"
REMOTE_USER="${REMOTE_USER:-root}"
REMOTE_PORT="${REMOTE_PORT:-22}"
SSH_KEY="${SSH_KEY:-$HOME/.ssh/id_ed25519_host}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

run_local() {
  local dc_dir="${DC_DIR:-}"
  if [ -z "$dc_dir" ]; then
    for d in "$REPO_ROOT" "$REPO_ROOT/../.." /root/lianel/dc /root/hosting-base/lianel/dc; do
      [ -f "${d}/.env" ] && [ -f "${d}/docker-compose.infra.yaml" ] && dc_dir="$d" && break
    done
  fi
  if [ -z "$dc_dir" ] || [ ! -f "$dc_dir/.env" ]; then
    echo "ERROR: Set DC_DIR or run from repo with .env (and docker-compose.infra.yaml)" >&2
    return 1
  fi
  export DC_DIR="$dc_dir"
  set -a
  # shellcheck source=/dev/null
  . "$dc_dir/.env"
  set +a
  IBKR_DIR="${dc_dir}/stock-service-ibkr"
  export IBKR_OAUTH_DH_PARAM_PATH="${IBKR_OAUTH_DH_PARAM_PATH:-$IBKR_DIR/dhparam.pem}"
  export IBKR_OAUTH_PRIVATE_ENCRYPTION_KEY_PATH="${IBKR_OAUTH_PRIVATE_ENCRYPTION_KEY_PATH:-$IBKR_DIR/private_encryption.pem}"
  export IBKR_OAUTH_PRIVATE_SIGNATURE_KEY_PATH="${IBKR_OAUTH_PRIVATE_SIGNATURE_KEY_PATH:-$IBKR_DIR/private_signature.pem}"
  export IBKR_OAUTH_REALM="${IBKR_OAUTH_REALM:-limited_poa}"
  export IBKR_API_BASE_URL="${IBKR_API_BASE_URL:-https://api.ibkr.com/v1/api}"
  if ! python3 "$SCRIPT_DIR/ibkr_get_live_session_token.py"; then
    return 1
  fi
  return 0
}

run_remote() {
  local ssh_opts=(-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=30 -p "$REMOTE_PORT")
  [ -n "${SSH_KEY:-}" ] && [ -f "$SSH_KEY" ] && ssh_opts+=(-i "$SSH_KEY")
  local scp_opts=(-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o ConnectTimeout=30 -P "$REMOTE_PORT")
  [ -n "${SSH_KEY:-}" ] && [ -f "$SSH_KEY" ] && scp_opts+=(-i "$SSH_KEY")
  # Copy Python script to remote /tmp so it runs even without git pull
  scp "${scp_opts[@]}" "$SCRIPT_DIR/ibkr_get_live_session_token.py" "${REMOTE_USER}@${REMOTE_HOST}:/tmp/ibkr_get_live_session_token.py"
  local remote_script="
    set -euo pipefail
    for d in /root/lianel/dc /root/hosting-base/lianel/dc; do
      [ -f \"\$d/.env\" ] && [ -f \"\$d/docker-compose.infra.yaml\" ] && DC_DIR=\"\$d\" && break
    done
    if [ -z \"\${DC_DIR:-}\" ]; then echo 'ERROR: No DC_DIR with .env found on remote'; exit 1; fi
    set -a && . \"\$DC_DIR/.env\" && set +a
    IBKR_DIR=\"\$DC_DIR/stock-service-ibkr\"
    export IBKR_OAUTH_DH_PARAM_PATH=\"\${IBKR_OAUTH_DH_PARAM_PATH:-\$IBKR_DIR/dhparam.pem}\"
    export IBKR_OAUTH_PRIVATE_ENCRYPTION_KEY_PATH=\"\${IBKR_OAUTH_PRIVATE_ENCRYPTION_KEY_PATH:-\$IBKR_DIR/private_encryption.pem}\"
    export IBKR_OAUTH_PRIVATE_SIGNATURE_KEY_PATH=\"\${IBKR_OAUTH_PRIVATE_SIGNATURE_KEY_PATH:-\$IBKR_DIR/private_signature.pem}\"
    export IBKR_OAUTH_REALM=\"\${IBKR_OAUTH_REALM:-limited_poa}\"
    export IBKR_API_BASE_URL=\"\${IBKR_API_BASE_URL:-https://api.ibkr.com/v1/api}\"
    if [ \"\${RUN_ONCE:-0}\" = \"1\" ]; then
      python3 /tmp/ibkr_get_live_session_token.py
    else
      until python3 /tmp/ibkr_get_live_session_token.py; do
        echo '--- Retry in 5s ---'
        sleep 5
      done
      echo 'Done: IBKR live session token OK'
    fi
  "
  ssh "${ssh_opts[@]}" "${REMOTE_USER}@${REMOTE_HOST}" "RUN_ONCE=${RUN_ONCE} bash -s" <<< "$remote_script"
}

if [ -n "$REMOTE_HOST" ]; then
  echo "Running on remote ${REMOTE_USER}@${REMOTE_HOST}..."
  [ "$RUN_ONCE" = "1" ] && export RUN_ONCE=1
  run_remote
else
  echo "Running locally..."
  if [ "$RUN_ONCE" = "1" ]; then
    run_local
  else
    until run_local; do
      echo "--- Retry in 5s ---"
      sleep 5
    done
    echo "Done: IBKR live session token OK"
  fi
fi
