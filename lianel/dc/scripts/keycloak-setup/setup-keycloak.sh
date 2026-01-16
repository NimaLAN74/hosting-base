#!/bin/bash

# Keycloak Setup Script
# This script configures Keycloak with the lianel realm and OAuth clients

set -e

KEYCLOAK_URL="http://keycloak:8080"
ADMIN_USER="admin"
ADMIN_PASS="D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA"

echo "=== Keycloak Setup Script ==="
echo

# Get admin token
echo "1. Getting admin access token..."
TOKEN=$(curl -s -X POST "${KEYCLOAK_URL}/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=${ADMIN_USER}" \
  -d "password=${ADMIN_PASS}" \
  -d 'grant_type=password' \
  -d 'client_id=admin-cli' | jq -r '.access_token')

if [ "$TOKEN" == "null" ] || [ -z "$TOKEN" ]; then
  echo "ERROR: Failed to get admin token"
  exit 1
fi
echo "✓ Got admin token"

# Create lianel realm
echo
echo "2. Creating 'lianel' realm..."
curl -s -X POST "${KEYCLOAK_URL}/admin/realms" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "realm": "lianel",
    "enabled": true,
    "displayName": "Lianel Services",
    "sslRequired": "external",
    "registrationAllowed": false,
    "loginWithEmailAllowed": true,
    "duplicateEmailsAllowed": false,
    "resetPasswordAllowed": true,
    "editUsernameAllowed": false,
    "bruteForceProtected": true
  }' && echo "✓ Realm created" || echo "⚠ Realm may already exist"

# Create OAuth2 Proxy client
echo
echo "3. Creating OAuth2 Proxy client..."
curl -s -X POST "${KEYCLOAK_URL}/admin/realms/lianel/clients" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "clientId": "oauth2-proxy",
    "enabled": true,
    "protocol": "openid-connect",
    "publicClient": false,
    "directAccessGrantsEnabled": false,
    "serviceAccountsEnabled": false,
    "authorizationServicesEnabled": false,
    "standardFlowEnabled": true,
    "implicitFlowEnabled": false,
    "redirectUris": [
      "https://auth.lianel.se/oauth2/callback",
      "https://*.lianel.se/*"
    ],
    "webOrigins": [
      "https://*.lianel.se"
    ],
    "attributes": {
      "pkce.code.challenge.method": "S256"
    }
  }' && echo "✓ OAuth2 Proxy client created"

# Get OAuth2 Proxy client secret
echo
echo "4. Getting OAuth2 Proxy client secret..."
CLIENT_UUID=$(curl -s "${KEYCLOAK_URL}/admin/realms/lianel/clients?clientId=oauth2-proxy" \
  -H "Authorization: Bearer ${TOKEN}" | jq -r '.[0].id')

OAUTH2_SECRET=$(curl -s "${KEYCLOAK_URL}/admin/realms/lianel/clients/${CLIENT_UUID}/client-secret" \
  -H "Authorization: Bearer ${TOKEN}" | jq -r '.value')

echo "✓ OAuth2 Proxy Client Secret: ${OAUTH2_SECRET}"
echo "  Add this to your .env file as OAUTH2_CLIENT_SECRET"

# Create Grafana client
echo
echo "5. Creating Grafana client..."
curl -s -X POST "${KEYCLOAK_URL}/admin/realms/lianel/clients" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "clientId": "grafana",
    "enabled": true,
    "protocol": "openid-connect",
    "publicClient": false,
    "directAccessGrantsEnabled": false,
    "standardFlowEnabled": true,
    "redirectUris": [
      "https://www.lianel.se/monitoring/*",
      "https://monitoring.lianel.se/*"
    ],
    "webOrigins": [
      "https://www.lianel.se",
      "https://monitoring.lianel.se"
    ]
  }' && echo "✓ Grafana client created"

# Get Grafana client secret
echo
echo "6. Getting Grafana client secret..."
GRAFANA_CLIENT_UUID=$(curl -s "${KEYCLOAK_URL}/admin/realms/lianel/clients?clientId=grafana" \
  -H "Authorization: Bearer ${TOKEN}" | jq -r '.[0].id')

GRAFANA_SECRET=$(curl -s "${KEYCLOAK_URL}/admin/realms/lianel/clients/${GRAFANA_CLIENT_UUID}/client-secret" \
  -H "Authorization: Bearer ${TOKEN}" | jq -r '.value')

echo "✓ Grafana Client Secret: ${GRAFANA_SECRET}"
echo "  Add this to your .env file as GRAFANA_OAUTH_CLIENT_SECRET"

# Create test user
echo
echo "7. Creating test user..."
curl -s -X POST "${KEYCLOAK_URL}/admin/realms/lianel/users" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "testuser",
    "email": "test@lianel.se",
    "enabled": true,
    "emailVerified": true,
    "credentials": [{
      "type": "password",
      "value": "Test123!",
      "temporary": false
    }]
  }' && echo "✓ Test user created (username: testuser, password: Test123!)"

echo
echo "=== Setup Complete! ==="
echo
echo "Next steps:"
echo "1. Update .env with the client secrets above"
echo "2. Restart OAuth2 Proxy: docker compose -f docker-compose.oauth2-proxy.yaml restart oauth2-proxy"
echo "3. Test SSO by visiting https://www.lianel.se/monitoring/"
