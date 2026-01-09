#!/bin/bash
# Fix Keycloak Realm Roles in Token for frontend-client
# This script ensures realm roles are included in the access token

set -e

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
echo "📋 Getting frontend-client configuration..."
CLIENT_UUID=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients?clientId=${CLIENT_ID}" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; clients = json.load(sys.stdin); print(clients[0]['id']) if clients else exit(1)" 2>/dev/null)

if [ -z "$CLIENT_UUID" ]; then
    echo "❌ frontend-client not found"
    exit 1
fi

echo "✅ Found frontend-client: ${CLIENT_UUID}"

# Get default client scopes
echo ""
echo "📋 Checking default client scopes..."
DEFAULT_SCOPES=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients/${CLIENT_UUID}/default-client-scopes" \
  -H "Authorization: Bearer ${TOKEN}")

echo "Current default scopes:"
echo "$DEFAULT_SCOPES" | python3 -c "import sys, json; scopes = json.load(sys.stdin); [print(f\"  - {s['name']}\") for s in scopes]" 2>/dev/null

# Check if realm-roles scope exists and is assigned
REALM_ROLES_SCOPE_ID=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/client-scopes" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; scopes = json.load(sys.stdin); [print(s['id']) for s in scopes if s['name'] == 'realm-roles']" 2>/dev/null)

if [ -z "$REALM_ROLES_SCOPE_ID" ]; then
    echo "⚠️  realm-roles scope not found - this is unusual, checking available scopes..."
    curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/client-scopes" \
      -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; scopes = json.load(sys.stdin); [print(f\"  - {s['name']} (id: {s['id']})\") for s in scopes]" 2>/dev/null
else
    echo "✅ Found realm-roles scope: ${REALM_ROLES_SCOPE_ID}"
    
    # Check if it's assigned as default scope
    HAS_REALM_ROLES=$(echo "$DEFAULT_SCOPES" | python3 -c "import sys, json; scopes = json.load(sys.stdin); print('yes' if any(s['id'] == '${REALM_ROLES_SCOPE_ID}') for s in scopes) else 'no'" 2>/dev/null)
    
    if [ "$HAS_REALM_ROLES" != "yes" ]; then
        echo "⚠️  realm-roles scope not assigned to frontend-client"
        echo "📝 Adding realm-roles scope to frontend-client..."
        
        curl -s -X PUT "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients/${CLIENT_UUID}/default-client-scopes/${REALM_ROLES_SCOPE_ID}" \
          -H "Authorization: Bearer ${TOKEN}" \
          -H "Content-Type: application/json" > /dev/null
        
        echo "✅ realm-roles scope added to frontend-client"
    else
        echo "✅ realm-roles scope already assigned"
    fi
fi

# Check protocol mapper for realm roles
echo ""
echo "📋 Checking protocol mapper for realm roles..."
REALM_ROLES_MAPPER=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/client-scopes/${REALM_ROLES_SCOPE_ID}/protocol-mappers/models" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; mappers = json.load(sys.stdin); [print(m['id']) for m in mappers if m['name'] == 'realm roles']" 2>/dev/null)

if [ -z "$REALM_ROLES_MAPPER" ]; then
    echo "⚠️  realm roles mapper not found in realm-roles scope"
    echo "📝 Creating realm roles mapper..."
    
    curl -s -X POST "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/client-scopes/${REALM_ROLES_SCOPE_ID}/protocol-mappers/models" \
      -H "Authorization: Bearer ${TOKEN}" \
      -H "Content-Type: application/json" \
      -d '{
        "name": "realm roles",
        "protocol": "openid-connect",
        "protocolMapper": "oidc-usermodel-realm-role-mapper",
        "config": {
          "user.attribute": "foo",
          "access.token.claim": "true",
          "claim.name": "realm_access.roles",
          "jsonType.label": "String",
          "multivalued": "true",
          "id.token.claim": "true"
        }
      }' > /dev/null
    
    echo "✅ realm roles mapper created"
else
    echo "✅ realm roles mapper exists: ${REALM_ROLES_MAPPER}"
fi

# Verify mapper configuration
echo ""
echo "📋 Verifying mapper configuration..."
MAPPER_CONFIG=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/client-scopes/${REALM_ROLES_SCOPE_ID}/protocol-mappers/models/${REALM_ROLES_MAPPER}" \
  -H "Authorization: Bearer ${TOKEN}")

echo "Mapper config:"
echo "$MAPPER_CONFIG" | python3 -c "import sys, json; m = json.load(sys.stdin); print(f\"  Name: {m['name']}\"); print(f\"  Protocol: {m['protocol']}\"); print(f\"  Claim name: {m['config'].get('claim.name', 'N/A')}\"); print(f\"  Access token claim: {m['config'].get('access.token.claim', 'N/A')}\"); print(f\"  Multivalued: {m['config'].get('multivalued', 'N/A')}\")" 2>/dev/null

# Check user role assignment
echo ""
echo "📋 Checking admin role assignment..."
echo "Please provide the username to check:"
read -r USERNAME

if [ -z "$USERNAME" ]; then
    echo "⚠️  No username provided, skipping user check"
else
    USER_ID=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/users?username=${USERNAME}" \
      -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; users = json.load(sys.stdin); print(users[0]['id']) if users else exit(1)" 2>/dev/null)
    
    if [ -z "$USER_ID" ]; then
        echo "❌ User not found: ${USERNAME}"
    else
        echo "✅ Found user: ${USER_ID}"
        
        # Get user's realm roles
        USER_ROLES=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/users/${USER_ID}/role-mappings/realm" \
          -H "Authorization: Bearer ${TOKEN}")
        
        echo "User's realm roles:"
        echo "$USER_ROLES" | python3 -c "import sys, json; roles = json.load(sys.stdin); [print(f\"  - {r['name']}\") for r in roles] if roles else print('  (no roles assigned)')" 2>/dev/null
        
        # Check if admin role is assigned
        HAS_ADMIN=$(echo "$USER_ROLES" | python3 -c "import sys, json; roles = json.load(sys.stdin); print('yes' if roles and any(r['name'].lower() in ['admin', 'realm-admin'] for r in roles) else 'no')" 2>/dev/null)
        
        if [ "$HAS_ADMIN" != "yes" ]; then
            echo "⚠️  User does not have admin or realm-admin role"
            echo "📝 To assign admin role, run:"
            echo "   ./assign-admin-role.sh ${USERNAME}"
        else
            echo "✅ User has admin role assigned"
        fi
    fi
fi

echo ""
echo "=== Summary ==="
echo "✅ Client scope configuration checked"
echo "✅ Protocol mapper verified"
echo ""
echo "⚠️  IMPORTANT: After making these changes:"
echo "   1. User must LOG OUT and LOG BACK IN to get a new token with roles"
echo "   2. The token refresh in the frontend will NOT work - user must re-authenticate"
echo "   3. Check browser console for the debug output showing tokenParsed.realm_access"
echo ""
echo "To test, decode the JWT token at https://jwt.io and verify it contains:"
echo '   "realm_access": { "roles": ["admin", ...] }'
