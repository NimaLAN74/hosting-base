#!/bin/bash

# Fix Keycloak HTTPS Configuration
# This script sets the frontend URL to HTTPS for the lianel realm

set -e

# Allow .env or env (e.g. from GitHub Actions) to supply admin password
[ -f ".env" ] && source .env
KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
ADMIN_USER="${KEYCLOAK_ADMIN_USER:-admin}"
ADMIN_PASS="${KEYCLOAK_ADMIN_PASSWORD:-D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA}"

echo "=== Fixing Keycloak HTTPS Configuration ==="
echo

# Get admin token
echo "1. Getting admin access token..."
TOKEN=$(curl -s -X POST "${KEYCLOAK_URL}/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=${ADMIN_USER}" \
  -d "password=${ADMIN_PASS}" \
  -d 'grant_type=password' \
  -d 'client_id=admin-cli' | python3 -c 'import sys, json; print(json.load(sys.stdin)["access_token"])')

if [ -z "$TOKEN" ] || [ "$TOKEN" == "null" ]; then
  echo "ERROR: Failed to get admin token"
  exit 1
fi
echo "✓ Got admin token"

# Get current realm configuration
echo
echo "2. Getting current realm configuration..."
CURRENT_REALM=$(curl -s "${KEYCLOAK_URL}/admin/realms/lianel" \
  -H "Authorization: Bearer ${TOKEN}")

# Realm frontendUrl: use app proxy (www.lianel.se/auth) so login page + theme + form POST
# are same-origin (www). Theme CSS and login-actions/authenticate then go via www's nginx
# to Keycloak, avoiding MIME type and 400 issues when Keycloak host was auth.lianel.se.
KEYCLOAK_FRONTEND_URL="${KEYCLOAK_FRONTEND_URL:-https://www.lianel.se/auth}"
export KEYCLOAK_FRONTEND_URL
echo
echo "3. Updating realm frontendUrl to Keycloak host: $KEYCLOAK_FRONTEND_URL..."
curl -s -X PUT "${KEYCLOAK_URL}/admin/realms/lianel" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d "$(echo "$CURRENT_REALM" | python3 -c '
import sys, os, json
realm = json.load(sys.stdin)
realm["attributes"] = realm.get("attributes", {})
realm["attributes"]["frontendUrl"] = os.environ.get("KEYCLOAK_FRONTEND_URL", "https://auth.lianel.se")
print(json.dumps(realm))
')" > /dev/null

echo "✓ Realm frontendUrl set to $KEYCLOAK_FRONTEND_URL (login + theme + POST via www same-origin)"

# Verify the update
echo
echo "4. Verifying update..."
UPDATED_REALM=$(curl -s "${KEYCLOAK_URL}/admin/realms/lianel" \
  -H "Authorization: Bearer ${TOKEN}")

REALM_FRONTEND=$(echo "$UPDATED_REALM" | python3 -c 'import sys, json; print(json.load(sys.stdin).get("attributes", {}).get("frontendUrl", "not set"))')
echo "Realm frontendUrl: ${REALM_FRONTEND}"

echo
echo "=== Update Complete! ==="

