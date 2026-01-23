#!/bin/bash
# Script to create Keycloak client for Comp AI Service
# Usage: ./create-comp-ai-keycloak-client.sh

set -e

# Load environment variables
if [ -f ".env" ]; then
    source .env
fi

KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
REALM_NAME="${KEYCLOAK_REALM:-lianel}"
ADMIN_USER="${KEYCLOAK_ADMIN_USER}"
ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD}"
CLIENT_ID="${COMP_AI_KEYCLOAK_CLIENT_ID:-comp-ai-service}"

if [ -z "$ADMIN_USER" ] || [ -z "$ADMIN_PASSWORD" ]; then
    echo "ERROR: KEYCLOAK_ADMIN_USER and KEYCLOAK_ADMIN_PASSWORD must be set"
    exit 1
fi

echo "=== Creating Keycloak Client for Comp AI Service ==="
echo "Keycloak URL: $KEYCLOAK_URL"
echo "Realm: $REALM_NAME"
echo "Client ID: $CLIENT_ID"
echo ""

# Get admin token
echo "1. Getting admin token..."
TOKEN_RESPONSE=$(curl -s -X POST "$KEYCLOAK_URL/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=admin-cli" \
  -d "username=$ADMIN_USER" \
  -d "password=$ADMIN_PASSWORD" \
  -d "grant_type=password")

TOKEN=$(echo "$TOKEN_RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin).get('access_token', ''))" 2>/dev/null)

if [ -z "$TOKEN" ] || [ "$TOKEN" == "null" ]; then
    echo "ERROR: Failed to get admin token"
    echo "Response: $TOKEN_RESPONSE"
    exit 1
fi

echo "   ✅ Admin token obtained"
echo ""

# Check if client already exists
echo "2. Checking if client already exists..."
EXISTING_CLIENT=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients?clientId=$CLIENT_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json")

CLIENT_UUID=$(echo "$EXISTING_CLIENT" | python3 -c "import sys, json; clients=json.load(sys.stdin); print(clients[0]['id'] if clients else '')" 2>/dev/null)

if [ -n "$CLIENT_UUID" ]; then
    echo "   ⚠️  Client already exists with UUID: $CLIENT_UUID"
    echo "   Updating existing client..."
    
    # Update existing client
    curl -s -X PUT "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$CLIENT_UUID" \
      -H "Authorization: Bearer $TOKEN" \
      -H "Content-Type: application/json" \
      -d '{
        "clientId": "'"$CLIENT_ID"'",
        "enabled": true,
        "clientAuthenticatorType": "client-secret",
        "secret": "'"${COMP_AI_KEYCLOAK_CLIENT_SECRET:-}"'",
        "redirectUris": [
          "https://www.lianel.se/api/v1/comp-ai/*",
          "https://lianel.se/api/v1/comp-ai/*"
        ],
        "webOrigins": [
          "https://www.lianel.se",
          "https://lianel.se"
        ],
        "standardFlowEnabled": true,
        "implicitFlowEnabled": false,
        "directAccessGrantsEnabled": true,
        "serviceAccountsEnabled": true,
        "publicClient": false,
        "protocol": "openid-connect",
        "attributes": {
          "access.token.lifespan": "300",
          "client.secret.creation.time": "'"$(date +%s)"'"
        }
      }' > /dev/null
    
    echo "   ✅ Client updated"
else
    echo "   Creating new client..."
    
    # Create new client
    CREATE_RESPONSE=$(curl -s -X POST "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients" \
      -H "Authorization: Bearer $TOKEN" \
      -H "Content-Type: application/json" \
      -d '{
        "clientId": "'"$CLIENT_ID"'",
        "enabled": true,
        "clientAuthenticatorType": "client-secret",
        "redirectUris": [
          "https://www.lianel.se/api/v1/comp-ai/*",
          "https://lianel.se/api/v1/comp-ai/*"
        ],
        "webOrigins": [
          "https://www.lianel.se",
          "https://lianel.se"
        ],
        "standardFlowEnabled": true,
        "implicitFlowEnabled": false,
        "directAccessGrantsEnabled": true,
        "serviceAccountsEnabled": true,
        "publicClient": false,
        "protocol": "openid-connect",
        "attributes": {
          "access.token.lifespan": "300"
        }
      }')
    
    # Get the created client UUID
    CLIENT_UUID=$(echo "$CREATE_RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin).get('id', ''))" 2>/dev/null || echo "")
    
    if [ -z "$CLIENT_UUID" ]; then
        echo "   ⚠️  Client may have been created, but UUID not found in response"
        echo "   Response: $CREATE_RESPONSE"
        # Try to get UUID by querying again
        EXISTING_CLIENT=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients?clientId=$CLIENT_ID" \
          -H "Authorization: Bearer $TOKEN" \
          -H "Content-Type: application/json")
        CLIENT_UUID=$(echo "$EXISTING_CLIENT" | python3 -c "import sys, json; clients=json.load(sys.stdin); print(clients[0]['id'] if clients else '')" 2>/dev/null)
    fi
    
    echo "   ✅ Client created with UUID: $CLIENT_UUID"
fi

echo ""

# Get client secret
echo "3. Getting client secret..."
SECRET_RESPONSE=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$CLIENT_UUID/client-secret" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json")

CLIENT_SECRET=$(echo "$SECRET_RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin).get('value', ''))" 2>/dev/null)

if [ -z "$CLIENT_SECRET" ] || [ "$CLIENT_SECRET" == "null" ]; then
    echo "   ⚠️  Could not retrieve client secret. You may need to generate it manually in Keycloak admin console."
    echo "   Response: $SECRET_RESPONSE"
else
    echo "   ✅ Client secret obtained"
    echo ""
    echo "=== Client Configuration ==="
    echo "Client ID: $CLIENT_ID"
    echo "Client Secret: $CLIENT_SECRET"
    echo ""
    echo "⚠️  IMPORTANT: Add this to your .env file:"
    echo "COMP_AI_KEYCLOAK_CLIENT_ID=$CLIENT_ID"
    echo "COMP_AI_KEYCLOAK_CLIENT_SECRET=$CLIENT_SECRET"
    echo ""
fi

echo "=== Keycloak Client Setup Complete ==="
