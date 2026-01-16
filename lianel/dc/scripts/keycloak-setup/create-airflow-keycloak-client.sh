#!/bin/bash

# Create Airflow Keycloak Client
# This script creates an OAuth client in Keycloak for Airflow SSO

set -e

KEYCLOAK_URL="https://auth.lianel.se"
ADMIN_USER="admin"
ADMIN_PASS="D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA"

echo "=== Creating Airflow Keycloak Client ==="
echo

# Get admin token
echo "1. Getting admin access token..."
TOKEN=$(curl -s -X POST "${KEYCLOAK_URL}/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=${ADMIN_USER}" \
  -d "password=${ADMIN_PASS}" \
  -d 'grant_type=password' \
  -d 'client_id=admin-cli' | python3 -c 'import sys, json; print(json.load(sys.stdin)["access_token"])')

if [ -z "$TOKEN" ] || [ "$TOKEN" == "null" ]; then
  echo "ERROR: Failed to get admin token"
  exit 1
fi
echo "✓ Got admin token"

# Check if client already exists
echo
echo "2. Checking if Airflow client already exists..."
EXISTING_CLIENT=$(curl -s "${KEYCLOAK_URL}/admin/realms/lianel/clients?clientId=airflow" \
  -H "Authorization: Bearer ${TOKEN}")

if [ "$(echo "$EXISTING_CLIENT" | python3 -c 'import sys, json; print(len(json.load(sys.stdin)))')" != "0" ]; then
  echo "⚠ Airflow client already exists. Updating..."
  CLIENT_ID=$(echo "$EXISTING_CLIENT" | python3 -c 'import sys, json; print(json.load(sys.stdin)[0]["id"])')
  
  # Update existing client
  curl -s -X PUT "${KEYCLOAK_URL}/admin/realms/lianel/clients/${CLIENT_ID}" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d '{
      "clientId": "airflow",
      "enabled": true,
      "protocol": "openid-connect",
      "publicClient": false,
      "directAccessGrantsEnabled": false,
      "serviceAccountsEnabled": false,
      "standardFlowEnabled": true,
      "implicitFlowEnabled": false,
      "redirectUris": [
        "https://airflow.lianel.se/oauth-authorized/keycloak"
      ],
      "webOrigins": [
        "https://airflow.lianel.se"
      ],
      "attributes": {
        "pkce.code.challenge.method": "S256"
      }
    }' > /dev/null
  echo "✓ Airflow client updated"
else
  # Create new client
  echo "3. Creating Airflow client..."
  curl -s -X POST "${KEYCLOAK_URL}/admin/realms/lianel/clients" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d '{
      "clientId": "airflow",
      "enabled": true,
      "protocol": "openid-connect",
      "publicClient": false,
      "directAccessGrantsEnabled": false,
      "serviceAccountsEnabled": false,
      "standardFlowEnabled": true,
      "implicitFlowEnabled": false,
      "redirectUris": [
        "https://airflow.lianel.se/oauth-authorized/keycloak"
      ],
      "webOrigins": [
        "https://airflow.lianel.se"
      ],
      "attributes": {
        "pkce.code.challenge.method": "S256"
      }
    }' > /dev/null && echo "✓ Airflow client created"
  
  CLIENT_ID=$(curl -s "${KEYCLOAK_URL}/admin/realms/lianel/clients?clientId=airflow" \
    -H "Authorization: Bearer ${TOKEN}" | python3 -c 'import sys, json; print(json.load(sys.stdin)[0]["id"])')
fi

# Get client secret
echo
echo "4. Getting Airflow client secret..."
AIRFLOW_SECRET=$(curl -s "${KEYCLOAK_URL}/admin/realms/lianel/clients/${CLIENT_ID}/client-secret" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c 'import sys, json; print(json.load(sys.stdin)["value"])')

echo "✓ Airflow Client Secret: ${AIRFLOW_SECRET}"
echo
echo "=== IMPORTANT ==="
echo "Add this to your .env file as AIRFLOW_OAUTH_CLIENT_SECRET=${AIRFLOW_SECRET}"
echo
echo "=== Client Configuration Complete! ==="

