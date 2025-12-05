#!/bin/bash

# Fix OAuth2-Proxy Client Redirect URIs
# This script updates the oauth2-proxy client in Keycloak to include all necessary redirect URIs

set -e

KEYCLOAK_URL="https://auth.lianel.se"
ADMIN_USER="admin"
ADMIN_PASS="D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA"

echo "=== Fixing OAuth2-Proxy Client Configuration ==="
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

# Get client ID
echo
echo "2. Getting OAuth2-Proxy client ID..."
CLIENT_ID=$(curl -s "${KEYCLOAK_URL}/admin/realms/lianel/clients?clientId=oauth2-proxy" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c 'import sys, json; print(json.load(sys.stdin)[0]["id"])')

if [ -z "$CLIENT_ID" ] || [ "$CLIENT_ID" == "null" ]; then
  echo "ERROR: Failed to get client ID"
  exit 1
fi
echo "✓ Client ID: ${CLIENT_ID}"

# Get current client configuration
echo
echo "3. Getting current client configuration..."
CURRENT_CONFIG=$(curl -s "${KEYCLOAK_URL}/admin/realms/lianel/clients/${CLIENT_ID}" \
  -H "Authorization: Bearer ${TOKEN}")

# Update client with all redirect URIs
echo
echo "4. Updating client with redirect URIs..."
curl -s -X PUT "${KEYCLOAK_URL}/admin/realms/lianel/clients/${CLIENT_ID}" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d "{
    \"redirectUris\": [
      \"https://lianel.se/oauth2/callback\",
      \"https://www.lianel.se/oauth2/callback\",
      \"https://auth.lianel.se/oauth2/callback\",
      \"https://*.lianel.se/*\"
    ],
    \"webOrigins\": [
      \"https://lianel.se\",
      \"https://www.lianel.se\",
      \"https://auth.lianel.se\",
      \"https://*.lianel.se\"
    ]
  }" > /dev/null

echo "✓ Client updated successfully"

# Verify the update
echo
echo "5. Verifying update..."
UPDATED_CONFIG=$(curl -s "${KEYCLOAK_URL}/admin/realms/lianel/clients/${CLIENT_ID}" \
  -H "Authorization: Bearer ${TOKEN}")

REDIRECT_URIS=$(echo "$UPDATED_CONFIG" | python3 -c 'import sys, json; print("\\n".join(json.load(sys.stdin).get("redirectUris", [])))')
echo "Current redirect URIs:"
echo "$REDIRECT_URIS" | sed 's/^/  - /'

echo
echo "=== Update Complete! ==="

