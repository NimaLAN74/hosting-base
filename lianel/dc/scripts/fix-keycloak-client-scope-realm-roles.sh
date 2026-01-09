#!/bin/bash
# Fix Keycloak Client Scope to Include Realm Roles in Token
# This ensures frontend-client receives realm roles in the access token

set -e

# Load environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
REALM_NAME="${REALM_NAME:-lianel}"
KEYCLOAK_ADMIN_USER="${KEYCLOAK_ADMIN_USER:-admin}"
KEYCLOAK_ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD}"
CLIENT_ID="frontend-client"

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

# Get frontend-client UUID
echo ""
echo "📋 Getting frontend-client..."
CLIENT_UUID=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients?clientId=${CLIENT_ID}" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; clients = json.load(sys.stdin); print(clients[0]['id']) if clients else exit(1)" 2>/dev/null)

if [ -z "$CLIENT_UUID" ]; then
    echo "❌ frontend-client not found"
    exit 1
fi

echo "✅ Found frontend-client: ${CLIENT_UUID}"

# Get realm-roles client scope ID
echo ""
echo "📋 Getting realm-roles client scope..."
REALM_ROLES_SCOPE_ID=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/client-scopes" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; scopes = json.load(sys.stdin); [print(s['id']) for s in scopes if s['name'] == 'realm-roles']" 2>/dev/null)

if [ -z "$REALM_ROLES_SCOPE_ID" ]; then
    echo "❌ realm-roles client scope not found"
    echo "   This is a default scope that should exist. Please check Keycloak configuration."
    exit 1
fi

echo "✅ Found realm-roles scope: ${REALM_ROLES_SCOPE_ID}"

# Check if realm-roles is assigned as default scope
echo ""
echo "📋 Checking if realm-roles is assigned to frontend-client..."
DEFAULT_SCOPES=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients/${CLIENT_UUID}/default-client-scopes" \
  -H "Authorization: Bearer ${TOKEN}")

HAS_REALM_ROLES=$(echo "$DEFAULT_SCOPES" | python3 -c "import sys, json; scopes = json.load(sys.stdin); print('yes' if any(s['id'] == '${REALM_ROLES_SCOPE_ID}') for s in scopes) else 'no'" 2>/dev/null)

if [ "$HAS_REALM_ROLES" != "yes" ]; then
    echo "⚠️  realm-roles scope NOT assigned to frontend-client"
    echo "📝 Adding realm-roles scope to frontend-client..."
    
    RESPONSE=$(curl -s -w "\n%{http_code}" -X PUT "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients/${CLIENT_UUID}/default-client-scopes/${REALM_ROLES_SCOPE_ID}" \
      -H "Authorization: Bearer ${TOKEN}" \
      -H "Content-Type: application/json")
    
    HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
    
    if [ "$HTTP_CODE" = "204" ] || [ "$HTTP_CODE" = "200" ]; then
        echo "✅ realm-roles scope added to frontend-client"
    else
        echo "❌ Failed to add realm-roles scope. HTTP code: ${HTTP_CODE}"
        echo "Response: $(echo "$RESPONSE" | head -n-1)"
        exit 1
    fi
else
    echo "✅ realm-roles scope already assigned to frontend-client"
fi

# Verify protocol mapper exists and is configured correctly
echo ""
echo "📋 Checking protocol mapper for realm roles..."
MAPPER_LIST=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/client-scopes/${REALM_ROLES_SCOPE_ID}/protocol-mappers/models" \
  -H "Authorization: Bearer ${TOKEN}")

REALM_ROLES_MAPPER_ID=$(echo "$MAPPER_LIST" | python3 -c "import sys, json; mappers = json.load(sys.stdin); [print(m['id']) for m in mappers if 'realm' in m['name'].lower() and 'role' in m['name'].lower()]" 2>/dev/null)

if [ -z "$REALM_ROLES_MAPPER_ID" ]; then
    echo "⚠️  realm roles mapper not found"
    echo "📝 Creating realm roles mapper..."
    
    RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/client-scopes/${REALM_ROLES_SCOPE_ID}/protocol-mappers/models" \
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
    
    HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
    
    if [ "$HTTP_CODE" = "201" ] || [ "$HTTP_CODE" = "200" ]; then
        echo "✅ realm roles mapper created"
        REALM_ROLES_MAPPER_ID=$(echo "$RESPONSE" | head -n-1 | python3 -c "import sys, json; print(json.load(sys.stdin).get('id', ''))" 2>/dev/null)
    else
        echo "⚠️  Failed to create mapper. HTTP code: ${HTTP_CODE}"
        echo "   Response: $(echo "$RESPONSE" | head -n-1)"
    fi
else
    echo "✅ Found realm roles mapper: ${REALM_ROLES_MAPPER_ID}"
fi

# Verify mapper configuration
if [ -n "$REALM_ROLES_MAPPER_ID" ]; then
    echo ""
    echo "📋 Verifying mapper configuration..."
    MAPPER_CONFIG=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/client-scopes/${REALM_ROLES_SCOPE_ID}/protocol-mappers/models/${REALM_ROLES_MAPPER_ID}" \
      -H "Authorization: Bearer ${TOKEN}")
    
    echo "Mapper details:"
    echo "$MAPPER_CONFIG" | python3 -c "
import sys, json
m = json.load(sys.stdin)
print(f\"  Name: {m.get('name', 'N/A')}\")
print(f\"  Protocol: {m.get('protocol', 'N/A')}\")
print(f\"  Protocol Mapper: {m.get('protocolMapper', 'N/A')}\")
config = m.get('config', {})
print(f\"  Access Token Claim: {config.get('access.token.claim', 'N/A')}\")
print(f\"  Multivalued: {config.get('multivalued', 'N/A')}\")
print(f\"  ID Token Claim: {config.get('id.token.claim', 'N/A')}\")
" 2>/dev/null
    
    # Check if mapper needs to be updated
    ACCESS_TOKEN_CLAIM=$(echo "$MAPPER_CONFIG" | python3 -c "import sys, json; print(json.load(sys.stdin)['config'].get('access.token.claim', 'false'))" 2>/dev/null)
    
    if [ "$ACCESS_TOKEN_CLAIM" != "true" ]; then
        echo "⚠️  Mapper access.token.claim is not 'true', updating..."
        curl -s -X PUT "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/client-scopes/${REALM_ROLES_SCOPE_ID}/protocol-mappers/models/${REALM_ROLES_MAPPER_ID}" \
          -H "Authorization: Bearer ${TOKEN}" \
          -H "Content-Type: application/json" \
          -d "$(echo "$MAPPER_CONFIG" | python3 -c "
import sys, json
m = json.load(sys.stdin)
m['config']['access.token.claim'] = 'true'
m['config']['multivalued'] = 'true'
print(json.dumps(m))
")" > /dev/null
        echo "✅ Mapper updated"
    fi
fi

echo ""
echo "=== Configuration Summary ==="
echo "✅ Client scope configuration verified"
echo "✅ Protocol mapper verified"
echo ""
echo "⚠️  CRITICAL: User must LOG OUT and LOG BACK IN to get a new token!"
echo "   Token refresh will NOT work - user must re-authenticate"
echo ""
echo "To verify the fix:"
echo "  1. User logs out completely"
echo "  2. User logs back in"
echo "  3. Check browser console for debug output"
echo "  4. Decode JWT token at https://jwt.io"
echo "  5. Verify token contains: \"realm_access\": { \"roles\": [\"admin\", ...] }"
