#!/usr/bin/env bash
set -euo pipefail

# Create a Keycloak realm admin user for Lianel
# Usage:
#   ./create-admin-user.sh <username> <email> <password> [firstName] [lastName]
#
# Requirements:
# - Run on the host where docker-compose infra stack is running
# - 'keycloak' container name present (see docker-compose.infra.yaml)
# - KEYCLOAK_ADMIN_USER and KEYCLOAK_ADMIN_PASSWORD available in environment or .env file
#
# Notes:
# - This script executes kcadm.sh INSIDE the keycloak container
# - No secrets are stored in repo. Provide password via argument or prompt.

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required" >&2
  exit 1
fi

if ! docker ps --format '{{.Names}}' | grep -q '^keycloak$'; then
  echo "Keycloak container 'keycloak' not running. Start infra first." >&2
  exit 1
fi

USERNAME=${1:-}
EMAIL=${2:-}
PASSWORD=${3:-}
FIRST_NAME=${4:-}
LAST_NAME=${5:-}

if [[ -z "$USERNAME" || -z "$EMAIL" ]]; then
  echo "Usage: $0 <username> <email> <password> [firstName] [lastName]" >&2
  exit 1
fi

if [[ -z "${PASSWORD}" ]]; then
  read -r -s -p "Enter password for ${USERNAME}: " PASSWORD
  echo
fi

REALM=${REALM:-lianel}
SERVER_URL=${SERVER_URL:-http://localhost:8080}

# Login as admin to master realm using env vars from container
echo "[1/4] Authenticating admin..."
docker exec -e KEYCLOAK_ADMIN="$KEYCLOAK_ADMIN_USER" -e KEYCLOAK_ADMIN_PASSWORD="$KEYCLOAK_ADMIN_PASSWORD" keycloak \
  /opt/keycloak/bin/kcadm.sh config credentials --server "$SERVER_URL" --realm master \
  --user "$KEYCLOAK_ADMIN" --password "$KEYCLOAK_ADMIN_PASSWORD" >/dev/null

# Create user (idempotent on username)
echo "[2/4] Creating user '${USERNAME}' in realm '${REALM}'..."
set +e
USER_JSON=$(docker exec keycloak /opt/keycloak/bin/kcadm.sh get users -r "$REALM" -q username="$USERNAME")
set -e
# Extract first id from JSON without jq (best-effort)
USER_ID=$(echo "$USER_JSON" | sed -n 's/.*"id"\s*:\s*"\([^"]\+\)".*/\1/p' | head -n1)
if [[ -z "$USER_ID" ]]; then
  docker exec keycloak /opt/keycloak/bin/kcadm.sh create users -r "$REALM" \
    -s username="$USERNAME" -s enabled=true -s email="$EMAIL" \
    ${FIRST_NAME:+-s firstName="$FIRST_NAME"} ${LAST_NAME:+-s lastName="$LAST_NAME"} >/dev/null
  USER_JSON=$(docker exec keycloak /opt/keycloak/bin/kcadm.sh get users -r "$REALM" -q username="$USERNAME")
  USER_ID=$(echo "$USER_JSON" | sed -n 's/.*"id"\s*:\s*"\([^"]\+\)".*/\1/p' | head -n1)
  echo "Created user with id: $USER_ID"
else
  echo "User already exists with id: $USER_ID"
fi

# Set password
echo "[3/4] Setting password (non-temporary)..."
docker exec keycloak /opt/keycloak/bin/kcadm.sh set-password -r "$REALM" --username "$USERNAME" --new-password "$PASSWORD" --temporary=false >/dev/null

# Assign 'admin' role (create if missing is out-of-scope; ensure role exists)
echo "[4/4] Assigning realm role 'admin'..."
set +e
docker exec keycloak /opt/keycloak/bin/kcadm.sh add-roles -r "$REALM" --uusername "$USERNAME" --rolename admin >/dev/null 2>&1
RC=$?
set -e
if [[ $RC -ne 0 ]]; then
  echo "Warning: Failed to assign 'admin' role. Ensure realm role 'admin' exists." >&2
else
  echo "Assigned 'admin' role to ${USERNAME}."
fi

echo "Done. Admin user '${USERNAME}' is ready."
