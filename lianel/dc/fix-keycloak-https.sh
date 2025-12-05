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

# Update realm with HTTPS frontend URL
echo
echo "3. Updating realm with HTTPS frontend URL..."
curl -s -X PUT "${KEYCLOAK_URL}/admin/realms/lianel" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d "$(echo "$CURRENT_REALM" | python3 -c "
import sys, json
realm = json.load(sys.stdin)
realm['attributes'] = realm.get('attributes', {})
realm['attributes']['frontendUrl'] = 'https://auth.lianel.se'
print(json.dumps(realm))
")" > /dev/null

echo "✓ Realm updated with HTTPS frontend URL"

# Verify the update
echo
echo "4. Verifying update..."
UPDATED_REALM=$(curl -s "${KEYCLOAK_URL}/admin/realms/lianel" \
  -H "Authorization: Bearer ${TOKEN}")

FRONTEND_URL=$(echo "$UPDATED_REALM" | python3 -c 'import sys, json; print(json.load(sys.stdin).get("attributes", {}).get("frontendUrl", "not set"))')
echo "Frontend URL: ${FRONTEND_URL}"

echo
echo "=== Update Complete! ==="

