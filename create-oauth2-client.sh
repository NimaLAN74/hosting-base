#!/bin/bash
set -e

TOKEN=$(curl -sk -X POST "https://auth.lianel.se/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=admin" \
  -d "password=D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA" \
  -d "grant_type=password" \
  -d "client_id=admin-cli" | jq -r ".access_token")

echo "=== Creating oauth2-proxy client ==="
curl -sk -X POST "https://auth.lianel.se/admin/realms/lianel/clients" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "clientId": "oauth2-proxy",
    "enabled": true,
    "protocol": "openid-connect",
    "publicClient": false,
    "directAccessGrantsEnabled": false,
    "standardFlowEnabled": true,
    "implicitFlowEnabled": false,
    "redirectUris": ["https://*.lianel.se/*"],
    "webOrigins": ["https://*.lianel.se"],
    "attributes": {"pkce.code.challenge.method": "S256"}
  }' && echo "✓ oauth2-proxy client created"

echo
echo "=== Getting oauth2-proxy client secret ==="
CLIENT_UUID=$(curl -sk "https://auth.lianel.se/admin/realms/lianel/clients?clientId=oauth2-proxy" \
  -H "Authorization: Bearer $TOKEN" | jq -r ".[0].id")

OAUTH2_SECRET=$(curl -sk "https://auth.lianel.se/admin/realms/lianel/clients/${CLIENT_UUID}/client-secret" \
  -H "Authorization: Bearer $TOKEN" | jq -r ".value")

echo "Client Secret: $OAUTH2_SECRET"
echo
echo "Update .env file with: OAUTH2_CLIENT_SECRET=$OAUTH2_SECRET"
