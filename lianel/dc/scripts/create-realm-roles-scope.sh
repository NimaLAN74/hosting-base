#!/bin/bash
# Create realm-roles client scope if it doesn't exist
# This scope is needed to include realm roles in access tokens

set -e

# Load environment variables
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
REALM_NAME="${REALM_NAME:-lianel}"
KEYCLOAK_ADMIN_USER="${KEYCLOAK_ADMIN_USER:-admin}"
KEYCLOAK_ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD}"

if [ -z "$KEYCLOAK_ADMIN_PASSWORD" ]; then
    echo "❌ KEYCLOAK_ADMIN_PASSWORD environment variable is required"
    exit 1
fi

echo "🔐 Getting admin token..."
TOKEN=$(curl -s -X POST "${KEYCLOAK_URL}/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=${KEYCLOAK_ADMIN_USER}" \
  -d "password=${KEYCLOAK_ADMIN_PASSWORD}" \
  -d "grant_type=password" \
  -d "client_id=admin-cli" | python3 -c "import sys, json; print(json.load(sys.stdin)['access_token'])" 2>/dev/null)

if [ -z "$TOKEN" ]; then
    echo "❌ Failed to get admin token"
    exit 1
fi

echo "✅ Admin token obtained"

# Check if realm-roles scope exists
echo ""
echo "📋 Checking for realm-roles scope..."
REALM_ROLES_SCOPE_ID=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/client-scopes" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; scopes = json.load(sys.stdin); [print(s['id']) for s in scopes if s['name'] == 'realm-roles']" 2>/dev/null)

if [ -n "$REALM_ROLES_SCOPE_ID" ]; then
    echo "✅ realm-roles scope already exists: ${REALM_ROLES_SCOPE_ID}"
    exit 0
fi

echo "⚠️  realm-roles scope not found, creating it..."

# Create realm-roles client scope
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/client-scopes" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "realm-roles",
    "description": "OpenID Connect scope for add user realm roles to the access token",
    "protocol": "openid-connect",
    "attributes": {
      "include.in.token.scope": "true",
      "display.on.consent.screen": "false",
      "consent.screen.text": ""
    }
  }')

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | head -n-1)

if [ "$HTTP_CODE" = "201" ] || [ "$HTTP_CODE" = "200" ]; then
    REALM_ROLES_SCOPE_ID=$(echo "$BODY" | python3 -c "import sys, json; print(json.load(sys.stdin).get('id', ''))" 2>/dev/null)
    echo "✅ realm-roles scope created: ${REALM_ROLES_SCOPE_ID}"
else
    echo "❌ Failed to create realm-roles scope. HTTP code: ${HTTP_CODE}"
    echo "Response: ${BODY}"
    exit 1
fi

# Create protocol mapper for realm roles
echo ""
echo "📋 Creating realm roles protocol mapper..."
MAPPER_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/client-scopes/${REALM_ROLES_SCOPE_ID}/protocol-mappers/models" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "realm roles",
    "protocol": "openid-connect",
    "protocolMapper": "oidc-usermodel-realm-role-mapper",
    "config": {
      "user.attribute": "",
      "access.token.claim": "true",
      "claim.name": "",
      "jsonType.label": "String",
      "multivalued": "true",
      "id.token.claim": "true"
    }
  }')

MAPPER_HTTP_CODE=$(echo "$MAPPER_RESPONSE" | tail -n1)

if [ "$MAPPER_HTTP_CODE" = "201" ] || [ "$MAPPER_HTTP_CODE" = "200" ]; then
    echo "✅ Realm roles protocol mapper created"
else
    echo "⚠️  Failed to create protocol mapper. HTTP code: ${MAPPER_HTTP_CODE}"
    echo "   You may need to create it manually in Keycloak Admin Console"
fi

echo ""
echo "✅ realm-roles scope setup complete"
echo ""
echo "Next: Run fix-keycloak-client-scope-realm-roles.sh to assign it to frontend-client"
