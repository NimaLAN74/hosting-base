#!/bin/bash
# Update frontend-client in Keycloak so PKCE is NOT required.
# Keycloak logs show: "PKCE enforced Client without code challenge method" and
# "Missing parameter: code_challenge_method" for frontend-client – login fails with 400.
# Run on server with KEYCLOAK_ADMIN_PASSWORD (or from Sync Nginx workflow).
set -e

KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
# When Keycloak is behind www proxy, use www so admin token request reaches Keycloak
[ -n "$KEYCLOAK_URL_WWW" ] && KEYCLOAK_URL="$KEYCLOAK_URL_WWW"
ADMIN_USER="${KEYCLOAK_ADMIN_USER:-admin}"
ADMIN_PASS="${KEYCLOAK_ADMIN_PASSWORD:-}"
if [ -z "$ADMIN_PASS" ]; then
  echo "ERROR: KEYCLOAK_ADMIN_PASSWORD not set. Export it or set in .env."
  exit 1
fi

echo "=== Updating frontend-client: PKCE not required ==="
echo "Keycloak: $KEYCLOAK_URL"
echo ""

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

echo ""
echo "2. Getting frontend-client..."
EXISTING=$(curl -s -k "${KEYCLOAK_URL}/admin/realms/lianel/clients?clientId=frontend-client" \
  -H "Authorization: Bearer ${TOKEN}")
CLIENT_ID=$(echo "$EXISTING" | python3 -c 'import sys, json; c=json.load(sys.stdin); print(c[0]["id"]) if c else exit(1)' 2>/dev/null || true)
if [ -z "$CLIENT_ID" ]; then
  echo "ERROR: frontend-client not found"
  exit 1
fi
echo "✓ Found frontend-client id: $CLIENT_ID"

echo ""
echo "3. Updating client (remove PKCE requirement)..."
TMP_JSON=$(mktemp)
trap "rm -f $TMP_JSON" EXIT
curl -s -k "${KEYCLOAK_URL}/admin/realms/lianel/clients/${CLIENT_ID}" -H "Authorization: Bearer ${TOKEN}" | python3 -c "
import sys, json
c = json.load(sys.stdin)
if c.get('attributes') is None:
  c['attributes'] = {}
c['attributes'].pop('pkce.code.challenge.required', None)
c['attributes']['pkce.code.challenge.method'] = ''
with open('$TMP_JSON', 'w') as f:
  json.dump(c, f)
" || { rm -f "$TMP_JSON"; exit 1; }

curl -s -k -X PUT "${KEYCLOAK_URL}/admin/realms/lianel/clients/${CLIENT_ID}" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d @"$TMP_JSON" > /dev/null
rm -f "$TMP_JSON"
trap - EXIT

echo "✓ frontend-client updated (PKCE not required)"
echo ""
echo "=== Done. Frontend login should work (no more Missing parameter: code_challenge_method). ==="
