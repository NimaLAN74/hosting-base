#!/bin/bash
# Configure Keycloak to send roles in ID token for Grafana OAuth
# This ensures Grafana can properly map Keycloak admin roles to Grafana Admin/GrafanaAdmin

set -e

KEYCLOAK_URL="${KEYCLOAK_URL:-http://keycloak:8080}"
REALM_NAME="${REALM_NAME:-lianel}"
ADMIN_USER="${KEYCLOAK_ADMIN:-admin}"
ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD}"

if [ -z "$ADMIN_PASSWORD" ]; then
  echo "Error: KEYCLOAK_ADMIN_PASSWORD not set"
  exit 1
fi

echo "=== Configuring Keycloak Roles for Grafana ==="
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
  echo "Error: Failed to get admin token"
  exit 1
fi

# Get grafana-client UUID
echo "Finding grafana-client..."
GRAFANA_CLIENT_UUID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients?clientId=grafana-client" \
  -H "Authorization: Bearer $TOKEN" | python3 -c "import sys, json; clients = json.load(sys.stdin); print(clients[0]['id'] if clients else '')" 2>/dev/null)

if [ -z "$GRAFANA_CLIENT_UUID" ]; then
  echo "Error: grafana-client not found"
  exit 1
fi

echo "Found grafana-client with UUID: $GRAFANA_CLIENT_UUID"

# Check if Realm Roles mapper exists
echo "Checking for Realm Roles mapper..."
EXISTING_MAPPER=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$GRAFANA_CLIENT_UUID/protocol-mappers/models" \
  -H "Authorization: Bearer $TOKEN" | python3 -c "
import sys, json
mappers = json.load(sys.stdin)
for m in mappers:
    if m.get('name') == 'realm roles':
        print(m['id'])
        break
" 2>/dev/null)

if [ -z "$EXISTING_MAPPER" ]; then
  echo "Creating Realm Roles mapper..."
  curl -s -X POST "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$GRAFANA_CLIENT_UUID/protocol-mappers/models" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "name": "realm roles",
      "protocol": "openid-connect",
      "protocolMapper": "oidc-usermodel-realm-role-mapper",
      "config": {
        "multivalued": "true",
        "user.attribute": "foo",
        "id.token.claim": "true",
        "access.token.claim": "true",
        "claim.name": "realm_access.roles",
        "jsonType.label": "String"
      }
    }' > /dev/null
  echo "✅ Realm Roles mapper created"
else
  echo "✅ Realm Roles mapper already exists"
fi

# Ensure roles scope is included in client scopes
echo "Checking client scopes..."
DEFAULT_CLIENT_SCOPES=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$GRAFANA_CLIENT_UUID/default-client-scopes" \
  -H "Authorization: Bearer $TOKEN" | python3 -c "
import sys, json
scopes = json.load(sys.stdin)
for s in scopes:
    if s.get('name') == 'roles':
        print('found')
        break
" 2>/dev/null)

if [ -z "$DEFAULT_CLIENT_SCOPES" ]; then
  echo "Adding 'roles' to default client scopes..."
  ROLES_SCOPE_ID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM_NAME/client-scopes" \
    -H "Authorization: Bearer $TOKEN" | python3 -c "
import sys, json
scopes = json.load(sys.stdin)
for s in scopes:
    if s.get('name') == 'roles':
        print(s['id'])
        break
" 2>/dev/null)
  
  if [ -n "$ROLES_SCOPE_ID" ]; then
    curl -s -X PUT "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$GRAFANA_CLIENT_UUID/default-client-scopes/$ROLES_SCOPE_ID" \
      -H "Authorization: Bearer $TOKEN" > /dev/null
    echo "✅ Added 'roles' scope"
  else
    echo "⚠️  Warning: 'roles' scope not found in realm"
  fi
else
  echo "✅ 'roles' scope already included"
fi

echo ""
echo "=== Configuration Complete ==="
echo ""
echo "Next steps:"
echo "1. Restart Grafana to pick up the new configuration"
echo "2. Log out and log back in to Grafana"
echo "3. Verify you can see Configuration menu and Connections page"
