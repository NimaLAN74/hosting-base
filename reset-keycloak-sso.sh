#!/bin/bash
# Complete Keycloak & SSO Reset Script
# This script resets everything: realm, users, clients, and SSO configuration

set -e

KEYCLOAK_URL="https://auth.lianel.se"
KEYCLOAK_INTERNAL_URL="http://keycloak:8080"
REALM_NAME="lianel"
ADMIN_USER="admin"
ADMIN_PASSWORD="D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA"

echo "=========================================="
echo "🔧 Keycloak & SSO Complete Reset"
echo "=========================================="
echo ""

# Check if Keycloak is accessible
echo "[1/10] Checking Keycloak availability..."
if ! curl -sk -f "${KEYCLOAK_URL}/realms/master/.well-known/openid-configuration" >/dev/null 2>&1; then
  echo "❌ Error: Cannot reach Keycloak at ${KEYCLOAK_URL}"
  echo "   Make sure Keycloak container is running"
  exit 1
fi
echo "✅ Keycloak is accessible"

# Get admin token
echo ""
echo "[2/10] Getting admin token..."
TOKEN=$(curl -sk -X POST "${KEYCLOAK_URL}/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=${ADMIN_USER}" \
  -d "password=${ADMIN_PASSWORD}" \
  -d "grant_type=password" \
  -d "client_id=admin-cli" | jq -r '.access_token')

if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
  echo "❌ Error: Failed to get admin token"
  exit 1
fi
echo "✅ Admin token obtained"

# Delete existing realm (if exists)
echo ""
echo "[3/10] Deleting existing '${REALM_NAME}' realm (if exists)..."
REALM_EXISTS=$(curl -sk -X GET "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}" \
  -H "Authorization: Bearer ${TOKEN}" -o /dev/null -w "%{http_code}" 2>/dev/null || echo "404")

if [ "$REALM_EXISTS" == "200" ]; then
  curl -sk -X DELETE "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}" \
    -H "Authorization: Bearer ${TOKEN}" >/dev/null
  echo "✅ Existing realm deleted"
  sleep 2  # Wait for deletion to complete
else
  echo "ℹ️  Realm doesn't exist, skipping deletion"
fi

# Create new realm
echo ""
echo "[4/10] Creating new '${REALM_NAME}' realm..."
curl -sk -X POST "${KEYCLOAK_URL}/admin/realms" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d "{
    \"realm\": \"${REALM_NAME}\",
    \"enabled\": true,
    \"displayName\": \"Lianel Services\",
    \"sslRequired\": \"external\",
    \"registrationAllowed\": false,
    \"loginWithEmailAllowed\": true,
    \"duplicateEmailsAllowed\": false,
    \"resetPasswordAllowed\": true,
    \"editUsernameAllowed\": false,
    \"bruteForceProtected\": true,
    \"accessTokenLifespan\": 300,
    \"ssoSessionIdleTimeout\": 1800,
    \"ssoSessionMaxLifespan\": 36000
  }" >/dev/null
echo "✅ Realm created"

# Create 'admin' realm role
echo ""
echo "[5/10] Creating 'admin' realm role..."
curl -sk -X POST "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/roles" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d "{\"name\": \"admin\"}" >/dev/null 2>&1 || echo "ℹ️  Admin role may already exist"
echo "✅ Admin role ready"

# Create OAuth2 Proxy client
echo ""
echo "[6/10] Creating OAuth2 Proxy client..."
OAUTH2_CLIENT_UUID=$(curl -sk -X POST "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "clientId": "oauth2-proxy",
    "enabled": true,
    "protocol": "openid-connect",
    "publicClient": false,
    "directAccessGrantsEnabled": false,
    "serviceAccountsEnabled": false,
    "standardFlowEnabled": true,
    "implicitFlowEnabled": false,
    "redirectUris": [
      "https://auth.lianel.se/oauth2/callback",
      "https://*.lianel.se/*",
      "https://lianel.se/*",
      "https://www.lianel.se/*"
    ],
    "webOrigins": [
      "https://*.lianel.se",
      "https://lianel.se",
      "https://www.lianel.se"
    ],
    "attributes": {
      "pkce.code.challenge.method": "S256"
    }
  }' | jq -r '.id')

OAUTH2_SECRET=$(curl -sk "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients/${OAUTH2_CLIENT_UUID}/client-secret" \
  -H "Authorization: Bearer ${TOKEN}" | jq -r '.value')
echo "✅ OAuth2 Proxy client created"
echo "   Secret: ${OAUTH2_SECRET}"

# Create Frontend client (public)
echo ""
echo "[7/10] Creating Frontend client..."
FRONTEND_CLIENT_UUID=$(curl -sk -X POST "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "clientId": "frontend-client",
    "name": "Frontend Client",
    "enabled": true,
    "publicClient": true,
    "standardFlowEnabled": true,
    "directAccessGrantsEnabled": true,
    "redirectUris": [
      "https://lianel.se",
      "https://lianel.se/*",
      "https://www.lianel.se",
      "https://www.lianel.se/*",
      "http://localhost:3000",
      "http://localhost:3000/*"
    ],
    "webOrigins": ["*"],
    "frontchannelLogout": true,
    "attributes": {
      "backchannel.logout.session.required": "true",
      "backchannel.logout.revoke.offline.tokens": "false",
      "pkce.code.challenge.method": "S256"
    }
  }' | jq -r '.id')
echo "✅ Frontend client created"

# Create Backend API client
echo ""
echo "[8/10] Creating Backend API client..."
BACKEND_CLIENT_UUID=$(curl -sk -X POST "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "clientId": "backend-api",
    "enabled": true,
    "publicClient": false,
    "standardFlowEnabled": false,
    "implicitFlowEnabled": false,
    "directAccessGrantsEnabled": false,
    "serviceAccountsEnabled": true,
    "protocol": "openid-connect",
    "attributes": {
      "access.token.signed.response.alg": "RS256"
    },
    "fullScopeAllowed": true
  }' | jq -r '.id')

BACKEND_SECRET=$(curl -sk "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients/${BACKEND_CLIENT_UUID}/client-secret" \
  -H "Authorization: Bearer ${TOKEN}" | jq -r '.value')
echo "✅ Backend API client created"
echo "   Secret: ${BACKEND_SECRET}"

# Create Grafana client
echo ""
echo "[9/10] Creating Grafana client..."
GRAFANA_CLIENT_UUID=$(curl -sk -X POST "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "clientId": "grafana",
    "enabled": true,
    "publicClient": false,
    "standardFlowEnabled": true,
    "implicitFlowEnabled": false,
    "directAccessGrantsEnabled": false,
    "redirectUris": [
      "https://monitoring.lianel.se/login/generic_oauth",
      "https://www.lianel.se/monitoring/*"
    ],
    "webOrigins": [
      "https://monitoring.lianel.se",
      "https://www.lianel.se"
    ],
    "protocol": "openid-connect",
    "fullScopeAllowed": true
  }' | jq -r '.id')

GRAFANA_SECRET=$(curl -sk "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients/${GRAFANA_CLIENT_UUID}/client-secret" \
  -H "Authorization: Bearer ${TOKEN}" | jq -r '.value')
echo "✅ Grafana client created"
echo "   Secret: ${GRAFANA_SECRET}"

# Create users
echo ""
echo "[10/10] Creating users..."

# Create admin user
echo "  Creating admin user..."
ADMIN_USER_ID=$(curl -sk -X POST "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/users" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d "{
    \"username\": \"admin\",
    \"email\": \"admin@lianel.se\",
    \"enabled\": true,
    \"emailVerified\": true,
    \"firstName\": \"Admin\",
    \"lastName\": \"User\",
    \"credentials\": [{
      \"type\": \"password\",
      \"value\": \"${ADMIN_PASSWORD}\",
      \"temporary\": false
    }]
  }" | jq -r '.id')

# Assign admin role to admin user
curl -sk -X POST "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/users/${ADMIN_USER_ID}/role-mappings/realm" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d "[{\"id\":\"$(curl -sk "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/roles/admin" -H "Authorization: Bearer ${TOKEN}" | jq -r '.id')\",\"name\":\"admin\"}]" >/dev/null
echo "  ✅ Admin user created with admin role"

# Create test user
echo "  Creating test user..."
curl -sk -X POST "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/users" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d "{
    \"username\": \"testuser\",
    \"email\": \"test@lianel.se\",
    \"enabled\": true,
    \"emailVerified\": true,
    \"firstName\": \"Test\",
    \"lastName\": \"User\",
    \"credentials\": [{
      \"type\": \"password\",
      \"value\": \"Test123!\",
      \"temporary\": false
    }]
  }" >/dev/null
echo "  ✅ Test user created"

echo ""
echo "=========================================="
echo "✅ Reset Complete!"
echo "=========================================="
echo ""
echo "📋 Client Secrets (update .env file):"
echo "  OAUTH2_CLIENT_SECRET=${OAUTH2_SECRET}"
echo "  BACKEND_CLIENT_SECRET=${BACKEND_SECRET}"
echo "  GRAFANA_OAUTH_CLIENT_SECRET=${GRAFANA_SECRET}"
echo ""
echo "👤 Admin Credentials:"
echo "  Username: admin"
echo "  Password: ${ADMIN_PASSWORD}"
echo "  Realm: ${REALM_NAME}"
echo ""
echo "🧪 Test User:"
echo "  Username: testuser"
echo "  Password: Test123!"
echo ""
echo "🔄 Next Steps:"
echo "  1. Update /root/lianel/dc/.env with new secrets above"
echo "  2. Restart OAuth2 Proxy: docker compose -f docker-compose.oauth2-proxy.yaml restart oauth2-proxy"
echo "  3. Test login at: https://lianel.se"
echo ""

