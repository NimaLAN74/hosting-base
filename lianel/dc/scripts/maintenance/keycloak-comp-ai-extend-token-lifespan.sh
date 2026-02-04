#!/usr/bin/env bash
# Set comp-ai-service Keycloak client access token lifespan to 1 hour (3600s).
# Run on server after sourcing .env (KEYCLOAK_URL, KEYCLOAK_ADMIN_USER, KEYCLOAK_ADMIN_PASSWORD).
# Usage: cd /root/lianel/dc && source .env 2>/dev/null; bash scripts/maintenance/keycloak-comp-ai-extend-token-lifespan.sh
set -e

KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
REALM="${KEYCLOAK_REALM:-lianel}"
CLIENT_ID="${COMP_AI_KEYCLOAK_CLIENT_ID:-comp-ai-service}"
LIFESPAN="${COMP_AI_TOKEN_LIFESPAN:-3600}"

if [ -z "${KEYCLOAK_ADMIN_USER:-}" ] || [ -z "${KEYCLOAK_ADMIN_PASSWORD:-}" ]; then
  echo "KEYCLOAK_ADMIN_USER and KEYCLOAK_ADMIN_PASSWORD required"
  exit 1
fi

ADMIN_RESP=$(curl -s -X POST "${KEYCLOAK_URL}/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=admin-cli" \
  -d "username=${KEYCLOAK_ADMIN_USER}" \
  -d "password=${KEYCLOAK_ADMIN_PASSWORD}" \
  -d "grant_type=password")
ADMIN_TOKEN=$(echo "$ADMIN_RESP" | python3 -c "import sys, json; print(json.load(sys.stdin).get('access_token', '') or '')" 2>/dev/null)
if [ -z "$ADMIN_TOKEN" ]; then
  echo "Failed to get admin token"
  exit 1
fi

CLIENTS_JSON=$(curl -s -X GET "${KEYCLOAK_URL}/admin/realms/${REALM}/clients?clientId=${CLIENT_ID}" \
  -H "Authorization: Bearer $ADMIN_TOKEN" -H "Content-Type: application/json")
CLIENT_UUID=$(echo "$CLIENTS_JSON" | python3 -c "import sys, json; c=json.load(sys.stdin); print(c[0]['id'] if c else '')" 2>/dev/null)
if [ -z "$CLIENT_UUID" ]; then
  echo "Client $CLIENT_ID not found"
  exit 1
fi

# GET full client, set attributes.access.token.lifespan, PUT back
CLIENT_JSON=$(curl -s -X GET "${KEYCLOAK_URL}/admin/realms/${REALM}/clients/${CLIENT_UUID}" \
  -H "Authorization: Bearer $ADMIN_TOKEN" -H "Content-Type: application/json")
# Ensure attributes exist and set access.token.lifespan
UPDATED=$(echo "$CLIENT_JSON" | python3 -c "
import sys, json
c = json.load(sys.stdin)
if 'attributes' not in c or c['attributes'] is None:
    c['attributes'] = {}
c['attributes']['access.token.lifespan'] = '$LIFESPAN'
print(json.dumps(c))
")
curl -s -X PUT "${KEYCLOAK_URL}/admin/realms/${REALM}/clients/${CLIENT_UUID}" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d "$UPDATED" > /dev/null
echo "Set $CLIENT_ID access.token.lifespan to ${LIFESPAN}s"
echo "Run set-comp-ai-airflow-token.sh to get a new long-lived token."