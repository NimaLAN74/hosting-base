#!/bin/bash

# Create Grafana Client in Keycloak
# This script creates the grafana-client needed for Grafana OAuth authentication

set -e

KEYCLOAK_URL="http://localhost:8080"
REALM_NAME="lianel"
ADMIN_USER="${KEYCLOAK_ADMIN_USER:-admin}"
ADMIN_PASS="${KEYCLOAK_ADMIN_PASSWORD:-D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA}"

echo "=== Creating Grafana Client in Keycloak ==="
echo ""

# Get admin token
echo "1. Getting admin access token..."
TOKEN=$(curl -s -X POST "${KEYCLOAK_URL}/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=${ADMIN_USER}" \
  -d "password=${ADMIN_PASS}" \
  -d 'grant_type=password' \
  -d 'client_id=admin-cli' | python3 -c "import sys, json; print(json.load(sys.stdin).get('access_token', ''))" 2>/dev/null)

if [ -z "$TOKEN" ] || [ "$TOKEN" == "None" ]; then
  echo "ERROR: Failed to get admin token"
  echo "Please check KEYCLOAK_ADMIN_PASSWORD is set correctly"
  exit 1
fi
echo "✓ Got admin token"
echo ""

# Check if client already exists
echo "2. Checking if grafana-client exists..."
EXISTING_CLIENT=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients?clientId=grafana-client" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; clients=json.load(sys.stdin); print(clients[0]['id']) if clients else print('')" 2>/dev/null)

if [ -n "$EXISTING_CLIENT" ]; then
  echo "⚠ grafana-client already exists (ID: $EXISTING_CLIENT)"
  echo "Updating redirect URIs..."
  
  # Update existing client
  curl -s -X PUT "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients/${EXISTING_CLIENT}" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d '{
      "clientId": "grafana-client",
      "enabled": true,
      "publicClient": false,
      "standardFlowEnabled": true,
      "implicitFlowEnabled": false,
      "directAccessGrantsEnabled": false,
      "serviceAccountsEnabled": false,
      "redirectUris": [
        "https://monitoring.lianel.se/login/generic_oauth"
      ],
      "webOrigins": [
        "https://monitoring.lianel.se"
      ],
      "protocol": "openid-connect",
      "attributes": {
        "pkce.code.challenge.method": "S256"
      },
      "fullScopeAllowed": true
    }' > /dev/null
  
  echo "✓ grafana-client updated"
else
  echo "Creating grafana-client..."
  
  # Create new client
  CLIENT_RESPONSE=$(curl -s -X POST "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d '{
      "clientId": "grafana-client",
      "enabled": true,
      "publicClient": false,
      "standardFlowEnabled": true,
      "implicitFlowEnabled": false,
      "directAccessGrantsEnabled": false,
      "serviceAccountsEnabled": false,
      "redirectUris": [
        "https://monitoring.lianel.se/login/generic_oauth"
      ],
      "webOrigins": [
        "https://monitoring.lianel.se"
      ],
      "protocol": "openid-connect",
      "attributes": {
        "pkce.code.challenge.method": "S256"
      },
      "fullScopeAllowed": true
    }')
  
  CLIENT_ID=$(echo "$CLIENT_RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin).get('id', ''))" 2>/dev/null || echo "")
  
  if [ -z "$CLIENT_ID" ]; then
    echo "ERROR: Failed to create client"
    echo "Response: $CLIENT_RESPONSE"
    exit 1
  fi
  
  echo "✓ grafana-client created (ID: $CLIENT_ID)"
fi

# Get client secret (needed for Grafana configuration)
echo ""
echo "3. Getting client secret..."
CLIENT_UUID=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients?clientId=grafana-client" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; clients=json.load(sys.stdin); print(clients[0]['id']) if clients else exit(1)" 2>/dev/null)

CLIENT_SECRET=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients/${CLIENT_UUID}/client-secret" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; print(json.load(sys.stdin).get('value', ''))" 2>/dev/null)

if [ -n "$CLIENT_SECRET" ] && [ "$CLIENT_SECRET" != "None" ]; then
  echo "✓ Client Secret: $CLIENT_SECRET"
  echo ""
  echo "⚠️  IMPORTANT: Update GRAFANA_OAUTH_CLIENT_SECRET in your .env file:"
  echo "   GRAFANA_OAUTH_CLIENT_SECRET=$CLIENT_SECRET"
else
  echo "⚠ Could not retrieve client secret (may be a public client or already configured)"
fi

echo ""
echo "=== Grafana Client Configuration Complete ==="
echo ""
echo "Next steps:"
echo "1. Update .env file with GRAFANA_OAUTH_CLIENT_SECRET if needed"
echo "2. Restart Grafana container: docker restart grafana"
echo "3. Access Grafana at: https://monitoring.lianel.se"
