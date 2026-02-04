#!/usr/bin/env bash
# Fetch a Comp-AI Bearer token from Keycloak (client_credentials) and set Airflow Variable COMP_AI_TOKEN.
# Run on the server where Airflow and .env live. Requires: KEYCLOAK_ADMIN_USER, KEYCLOAK_ADMIN_PASSWORD,
# COMP_AI_KEYCLOAK_CLIENT_SECRET (or we fetch secret via admin API), KEYCLOAK_URL.
# Usage: cd /root/lianel/dc && bash scripts/maintenance/set-comp-ai-airflow-token.sh
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DC_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$DC_DIR"

for env in .env ../.env /root/lianel/dc/.env /root/hosting-base/lianel/dc/.env; do
  [ -f "$env" ] && source "$env" && break
done

KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
REALM="${KEYCLOAK_REALM:-lianel}"
CLIENT_ID="${COMP_AI_KEYCLOAK_CLIENT_ID:-comp-ai-service}"

if [ -z "${KEYCLOAK_ADMIN_USER:-}" ] || [ -z "${KEYCLOAK_ADMIN_PASSWORD:-}" ]; then
  echo "ERROR: KEYCLOAK_ADMIN_USER and KEYCLOAK_ADMIN_PASSWORD must be set (e.g. in .env)"
  exit 1
fi

# 1) Admin token (master realm)
ADMIN_RESP=$(curl -s -X POST "${KEYCLOAK_URL}/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=admin-cli" \
  -d "username=${KEYCLOAK_ADMIN_USER}" \
  -d "password=${KEYCLOAK_ADMIN_PASSWORD}" \
  -d "grant_type=password")
ADMIN_TOKEN=$(echo "$ADMIN_RESP" | python3 -c "import sys, json; print(json.load(sys.stdin).get('access_token', '') or '')" 2>/dev/null)
if [ -z "$ADMIN_TOKEN" ]; then
  echo "ERROR: Failed to get Keycloak admin token"
  exit 1
fi

# 2) Client secret (if not in env)
CLIENT_SECRET="${COMP_AI_KEYCLOAK_CLIENT_SECRET:-}"
if [ -z "$CLIENT_SECRET" ]; then
  CLIENTS_JSON=$(curl -s -X GET "${KEYCLOAK_URL}/admin/realms/${REALM}/clients?clientId=${CLIENT_ID}" \
    -H "Authorization: Bearer $ADMIN_TOKEN" -H "Content-Type: application/json")
  CLIENT_UUID=$(echo "$CLIENTS_JSON" | python3 -c "import sys, json; c=json.load(sys.stdin); print(c[0]['id'] if c else '')" 2>/dev/null)
  if [ -z "$CLIENT_UUID" ]; then
    echo "ERROR: comp-ai client not found in Keycloak"
    exit 1
  fi
  SECRET_JSON=$(curl -s -X GET "${KEYCLOAK_URL}/admin/realms/${REALM}/clients/${CLIENT_UUID}/client-secret" \
    -H "Authorization: Bearer $ADMIN_TOKEN" -H "Content-Type: application/json")
  CLIENT_SECRET=$(echo "$SECRET_JSON" | python3 -c "import sys, json; print(json.load(sys.stdin).get('value', '') or '')" 2>/dev/null)
  if [ -z "$CLIENT_SECRET" ]; then
    echo "ERROR: Could not get client secret for $CLIENT_ID"
    exit 1
  fi
fi

# 3) Access token (client_credentials) - ASCII-only for HTTP headers
TOKEN_RESP=$(curl -s -X POST "${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=${CLIENT_ID}" \
  -d "client_secret=${CLIENT_SECRET}" \
  -d "grant_type=client_credentials")
TOKEN=$(echo "$TOKEN_RESP" | python3 -c "import sys, json; print(json.load(sys.stdin).get('access_token', '') or '')" 2>/dev/null)
if [ -z "$TOKEN" ]; then
  echo "ERROR: Failed to get Comp-AI access token"
  exit 1
fi

# 4) Set Airflow variable (pass token via file to avoid shell escaping)
CONTAINER=$(docker ps -q -f name=airflow-apiserver | head -1)
if [ -z "$CONTAINER" ]; then
  echo "ERROR: No airflow-apiserver container found"
  exit 1
fi
TMPF="/tmp/comp_ai_token.$$"
printf '%s' "$TOKEN" > "$TMPF"
docker exec "$CONTAINER" airflow variables set COMP_AI_BASE_URL "http://lianel-comp-ai-service:3002"
docker cp "$TMPF" "$CONTAINER:/tmp/comp_ai_token"
docker exec "$CONTAINER" bash -c 'airflow variables set COMP_AI_TOKEN "$(cat /tmp/comp_ai_token)"'
docker exec "$CONTAINER" rm -f /tmp/comp_ai_token 2>/dev/null || true
rm -f "$TMPF"
echo "COMP_AI_BASE_URL and COMP_AI_TOKEN set in Airflow."
