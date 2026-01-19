#!/bin/bash
# Update Keycloak frontend-client to ensure /monitoring redirect URI is allowed
# This script updates the frontend-client to include all necessary redirect URIs

set -e

KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
REALM_NAME="${REALM_NAME:-lianel}"
ADMIN_USER="${KEYCLOAK_ADMIN_USER:-admin}"
ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD:-D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA}"

echo "=== Updating Keycloak frontend-client ==="
echo "Keycloak URL: $KEYCLOAK_URL"
echo "Realm: $REALM_NAME"
echo ""

# Get admin token
echo "Getting admin token..."
TOKEN=$(curl -s -X POST "$KEYCLOAK_URL/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=$ADMIN_USER" \
  -d "password=$ADMIN_PASSWORD" \
  -d "grant_type=password" \
  -d "client_id=admin-cli" | python3 -c "import sys, json; print(json.load(sys.stdin)['access_token'])" 2>/dev/null)

if [ -z "$TOKEN" ]; then
  echo "ERROR: Failed to get admin token"
  exit 1
fi

echo "✓ Admin token obtained"

# Get frontend-client UUID
echo "Finding frontend-client..."
FRONTEND_CLIENT_UUID=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients?clientId=frontend-client" \
  -H "Authorization: Bearer $TOKEN" | python3 -c "import sys, json; clients = json.load(sys.stdin); print(clients[0]['id']) if clients else exit(1)" 2>/dev/null || echo "")

if [ -z "$FRONTEND_CLIENT_UUID" ]; then
  echo "ERROR: frontend-client not found"
  exit 1
fi

echo "✓ Frontend client found: $FRONTEND_CLIENT_UUID"

# Update frontend-client with comprehensive redirect URIs
echo "Updating frontend-client redirect URIs..."
curl -s -X PUT "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$FRONTEND_CLIENT_UUID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "redirectUris": [
      "https://www.lianel.se/*",
      "https://lianel.se/*",
      "https://www.lianel.se/monitoring",
      "https://www.lianel.se/monitoring/*",
      "https://lianel.se/monitoring",
      "https://lianel.se/monitoring/*"
    ],
    "webOrigins": [
      "https://www.lianel.se",
      "https://lianel.se"
    ],
    "attributes": {
      "post.logout.redirect.uris": "https://www.lianel.se\nhttps://lianel.se\nhttps://www.lianel.se/monitoring\nhttps://lianel.se/monitoring"
    }
  }' > /dev/null

echo "✅ Frontend client updated with /monitoring redirect URIs"
echo ""
echo "Updated redirect URIs:"
echo "  - https://www.lianel.se/*"
echo "  - https://www.lianel.se/monitoring"
echo "  - https://www.lianel.se/monitoring/*"
echo ""
echo "The login button should now work correctly and return to /monitoring after login."
