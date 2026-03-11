#!/bin/bash
# Update grafana-client in Keycloak so PKCE is NOT required (Grafana may not send code_challenge in some versions/flows).
# Same approach as create-airflow-keycloak-client.sh. Run on server with KEYCLOAK_ADMIN_PASSWORD.
set -e

KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
ADMIN_USER="${KEYCLOAK_ADMIN_USER:-admin}"
ADMIN_PASS="${KEYCLOAK_ADMIN_PASSWORD:-}"
if [ -z "$ADMIN_PASS" ]; then
  echo "ERROR: KEYCLOAK_ADMIN_PASSWORD not set. Export it or set in .env."
  exit 1
fi

echo "=== Updating grafana-client: PKCE not required ==="
echo "Keycloak: $KEYCLOAK_URL"
echo ""

# Get admin token
echo "1. Getting admin access token..."
TOKEN=$(curl -s -k -X POST "${KEYCLOAK_URL}/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=${ADMIN_USER}" \
  -d "password=${ADMIN_PASS}" \
  -d 'grant_type=password' \
  -d 'client_id=admin-cli' | python3 -c 'import sys, json; print(json.load(sys.stdin).get("access_token", ""))')

if [ -z "$TOKEN" ] || [ "$TOKEN" == "None" ]; then
  echo "ERROR: Failed to get admin token"
  exit 1
fi
echo "✓ Got admin token"

# Get grafana-client id
echo ""
echo "2. Getting grafana-client..."
EXISTING=$(curl -s -k "${KEYCLOAK_URL}/admin/realms/lianel/clients?clientId=grafana-client" \
  -H "Authorization: Bearer ${TOKEN}")
CLIENT_ID=$(echo "$EXISTING" | python3 -c 'import sys, json; c=json.load(sys.stdin); print(c[0]["id"]) if c else exit(1)' 2>/dev/null || true)
if [ -z "$CLIENT_ID" ]; then
  echo "ERROR: grafana-client not found"
  exit 1
fi
echo "✓ Found grafana-client id: $CLIENT_ID"

# GET full client, clear PKCE requirement, PUT
echo ""
echo "3. Updating client (remove PKCE requirement)..."
TMP_JSON=$(mktemp)
trap "rm -f $TMP_JSON" EXIT
curl -s -k "${KEYCLOAK_URL}/admin/realms/lianel/clients/${CLIENT_ID}" -H "Authorization: Bearer ${TOKEN}" | python3 -c "
import sys, json
c = json.load(sys.stdin)
if c.get('attributes') is None:
  c['attributes'] = {}
# Do not require PKCE (Grafana use_pkce may not send it in all flows)
c['attributes'].pop('pkce.code.challenge.required', None)
c['attributes']['pkce.code.challenge.method'] = ''   # empty = do not require PKCE
with open('$TMP_JSON', 'w') as f:
  json.dump(c, f)
" || { rm -f "$TMP_JSON"; exit 1; }

curl -s -k -X PUT "${KEYCLOAK_URL}/admin/realms/lianel/clients/${CLIENT_ID}" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d @"$TMP_JSON" > /dev/null
rm -f "$TMP_JSON"
trap - EXIT

echo "✓ grafana-client updated (PKCE not required)"
echo ""
echo "=== Done. Grafana login should work without PKCE. ==="
