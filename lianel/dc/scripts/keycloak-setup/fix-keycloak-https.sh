#!/bin/bash

# Fix Keycloak HTTPS Configuration
# This script sets the frontend URL to HTTPS for the lianel realm

set -e

KEYCLOAK_URL="https://auth.lianel.se"
ADMIN_USER="admin"
ADMIN_PASS="D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA"

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

# Update realm with app frontend URL (so post-login redirect goes to app, not auth.lianel.se/admin)
# Using https://auth.lianel.se made users land on Keycloak admin console instead of the app.
APP_FRONTEND_URL="${APP_FRONTEND_URL:-https://www.lianel.se}"
export APP_FRONTEND_URL
echo
echo "3. Updating realm frontendUrl to app URL: $APP_FRONTEND_URL..."
curl -s -X PUT "${KEYCLOAK_URL}/admin/realms/lianel" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d "$(echo "$CURRENT_REALM" | python3 -c '
import sys, os, json
realm = json.load(sys.stdin)
realm["attributes"] = realm.get("attributes", {})
realm["attributes"]["frontendUrl"] = os.environ.get("APP_FRONTEND_URL", "https://www.lianel.se")
print(json.dumps(realm))
')" > /dev/null

echo "✓ Realm frontendUrl set to $APP_FRONTEND_URL (post-login redirect will go to app)"

# Verify the update
echo
echo "4. Verifying update..."
UPDATED_REALM=$(curl -s "${KEYCLOAK_URL}/admin/realms/lianel" \
  -H "Authorization: Bearer ${TOKEN}")

FRONTEND_URL=$(echo "$UPDATED_REALM" | python3 -c 'import sys, json; print(json.load(sys.stdin).get("attributes", {}).get("frontendUrl", "not set"))')
echo "Frontend URL: ${FRONTEND_URL}"

echo
echo "=== Update Complete! ==="

