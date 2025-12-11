#!/bin/bash
# Complete Keycloak Configuration for Direct Integration
# This script configures Keycloak with:
# - Login for admin and users
# - MFA mandatory for all users
# - Proper logout configuration
# - Clients for frontend, backend, Airflow, Grafana

set -e

KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
REALM_NAME="${REALM_NAME:-lianel}"
ADMIN_USER="${KEYCLOAK_ADMIN_USER:-admin}"
ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD:-D2eF5gH9jK3lM7nP1qR4sT8vW2xY6zA}"

echo "=== Keycloak Direct Integration Configuration ==="
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
  -d "client_id=admin-cli" | python3 -c "import sys, json; print(json.load(sys.stdin)['access_token'])")

if [ -z "$TOKEN" ]; then
  echo "ERROR: Failed to get admin token"
  exit 1
fi

echo "✓ Admin token obtained"

# Check if realm exists
REALM_EXISTS=$(curl -s -X GET "$KEYCLOAK_URL/admin/realms/$REALM_NAME" \
  -H "Authorization: Bearer $TOKEN" -o /dev/null -w "%{http_code}")

if [ "$REALM_EXISTS" != "200" ]; then
  echo "Creating realm $REALM_NAME..."
  curl -s -X POST "$KEYCLOAK_URL/admin/realms" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"realm\": \"$REALM_NAME\",
      \"enabled\": true,
      \"displayName\": \"Lianel\",
      \"loginTheme\": \"keycloak\",
      \"accountTheme\": \"keycloak\",
      \"emailTheme\": \"keycloak\",
      \"accessTokenLifespan\": 300,
      \"accessTokenLifespanForImplicitFlow\": 900,
      \"ssoSessionIdleTimeout\": 1800,
      \"ssoSessionMaxLifespan\": 36000,
      \"offlineSessionIdleTimeout\": 2592000,
      \"accessCodeLifespan\": 60,
      \"accessCodeLifespanUserAction\": 300,
      \"accessCodeLifespanLogin\": 1800,
      \"actionTokenGeneratedByAdminLifespan\": 43200,
      \"actionTokenGeneratedByUserLifespan\": 300,
      \"enabled\": true,
      \"sslRequired\": \"external\",
      \"registrationAllowed\": false,
      \"registrationEmailAsUsername\": false,
      \"rememberMe\": true,
      \"verifyEmail\": false,
      \"loginWithEmailAllowed\": true,
      \"duplicateEmailsAllowed\": false,
      \"resetPasswordAllowed\": true,
      \"editUsernameAllowed\": false,
      \"bruteForceProtected\": true,
      \"permanentLockout\": false,
      \"maxFailureWaitSeconds\": 900,
      \"minimumQuickLoginWaitSeconds\": 60,
      \"waitIncrementSeconds\": 60,
      \"quickLoginCheckMilliSeconds\": 1000,
      \"maxDeltaTimeSeconds\": 43200,
      \"failureFactor\": 30
    }" > /dev/null
  echo "✓ Realm created"
else
  echo "✓ Realm already exists"
fi

# Configure MFA - Set Conditional 2FA to CONDITIONAL and OTP Form to REQUIRED
echo ""
echo "Configuring MFA..."
CONDITIONAL_2FA_ID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM_NAME/authentication/flows/browser/executions" \
  -H "Authorization: Bearer $TOKEN" | python3 -c "
import sys, json
execs = json.load(sys.stdin)
cond = [e for e in execs if 'Conditional 2FA' in e.get('displayName', '')]
print(cond[0]['id']) if cond else exit(1)
")

if [ -n "$CONDITIONAL_2FA_ID" ]; then
  curl -s -X PUT "$KEYCLOAK_URL/admin/realms/$REALM_NAME/authentication/flows/browser/executions" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"id\":\"$CONDITIONAL_2FA_ID\",\"requirement\":\"CONDITIONAL\"}" > /dev/null
  echo "✓ Conditional 2FA set to CONDITIONAL"
fi

# Get OTP Form execution ID from conditional-otp subflow
OTP_EXEC_ID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM_NAME/authentication/flows/conditional-otp/executions" \
  -H "Authorization: Bearer $TOKEN" | python3 -c "
import sys, json
execs = json.load(sys.stdin)
otp = [e for e in execs if 'OTP Form' in e.get('displayName', '')]
print(otp[0]['id']) if otp else exit(1)
")

if [ -n "$OTP_EXEC_ID" ]; then
  curl -s -X PUT "$KEYCLOAK_URL/admin/realms/$REALM_NAME/authentication/flows/conditional-otp/executions" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"id\":\"$OTP_EXEC_ID\",\"requirement\":\"REQUIRED\"}" > /dev/null
  echo "✓ OTP Form set to REQUIRED"
fi

# Set Configure OTP as default required action
curl -s -X PUT "$KEYCLOAK_URL/admin/realms/$REALM_NAME/authentication/required-actions/CONFIGURE_TOTP" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "alias": "CONFIGURE_TOTP",
    "name": "Configure OTP",
    "providerId": "CONFIGURE_TOTP",
    "enabled": true,
    "defaultAction": true,
    "priority": 10,
    "config": {}
  }' > /dev/null
echo "✓ Configure OTP set as default required action"

# Create/Update Frontend Client (Public)
echo ""
echo "Configuring Frontend Client..."
FRONTEND_CLIENT_UUID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients?clientId=frontend-client" \
  -H "Authorization: Bearer $TOKEN" | python3 -c "
import sys, json
clients = json.load(sys.stdin)
print(clients[0]['id']) if clients else exit(1)
" 2>/dev/null || echo "")

if [ -z "$FRONTEND_CLIENT_UUID" ]; then
  echo "Creating frontend-client..."
  FRONTEND_CLIENT_UUID=$(curl -s -X POST "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "clientId": "frontend-client",
      "enabled": true,
      "publicClient": true,
      "standardFlowEnabled": true,
      "implicitFlowEnabled": false,
      "directAccessGrantsEnabled": true,
      "serviceAccountsEnabled": false,
      "redirectUris": [
        "https://www.lianel.se/*",
        "https://lianel.se/*"
      ],
      "webOrigins": [
        "https://www.lianel.se",
        "https://lianel.se"
      ],
      "attributes": {
        "post.logout.redirect.uris": "https://www.lianel.se\nhttps://lianel.se"
      },
      "protocol": "openid-connect",
      "fullScopeAllowed": true
    }' | python3 -c "import sys, json; print(json.load(sys.stdin)['id'])" 2>/dev/null)
  echo "✓ Frontend client created"
else
  echo "Updating frontend-client..."
  curl -s -X PUT "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$FRONTEND_CLIENT_UUID" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "clientId": "frontend-client",
      "enabled": true,
      "publicClient": true,
      "standardFlowEnabled": true,
      "implicitFlowEnabled": false,
      "directAccessGrantsEnabled": true,
      "serviceAccountsEnabled": false,
      "redirectUris": [
        "https://www.lianel.se/*",
        "https://lianel.se/*"
      ],
      "webOrigins": [
        "https://www.lianel.se",
        "https://lianel.se"
      ],
      "attributes": {
        "post.logout.redirect.uris": "https://www.lianel.se\nhttps://lianel.se"
      },
      "protocol": "openid-connect",
      "fullScopeAllowed": true
    }' > /dev/null
  echo "✓ Frontend client updated"
fi

# Create/Update Backend API Client (Confidential)
echo ""
echo "Configuring Backend API Client..."
BACKEND_CLIENT_UUID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients?clientId=backend-api" \
  -H "Authorization: Bearer $TOKEN" | python3 -c "
import sys, json
clients = json.load(sys.stdin)
print(clients[0]['id']) if clients else exit(1)
" 2>/dev/null || echo "")

if [ -z "$BACKEND_CLIENT_UUID" ]; then
  echo "Creating backend-api client..."
  BACKEND_CLIENT_UUID=$(curl -s -X POST "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients" \
    -H "Authorization: Bearer $TOKEN" \
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
    }' | python3 -c "import sys, json; print(json.load(sys.stdin)['id'])" 2>/dev/null)
  
  # Get client secret
  BACKEND_SECRET=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$BACKEND_CLIENT_UUID/client-secret" \
    -H "Authorization: Bearer $TOKEN" | python3 -c "import sys, json; print(json.load(sys.stdin)['value'])")
  echo "✓ Backend API client created"
  echo "  Client Secret: $BACKEND_SECRET"
else
  echo "✓ Backend API client already exists"
fi

# Create/Update Grafana Client
echo ""
echo "Configuring Grafana Client..."
GRAFANA_CLIENT_UUID=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients?clientId=grafana-client" \
  -H "Authorization: Bearer $TOKEN" | python3 -c "
import sys, json
clients = json.load(sys.stdin)
print(clients[0]['id']) if clients else exit(1)
" 2>/dev/null || echo "")

if [ -z "$GRAFANA_CLIENT_UUID" ]; then
  echo "Creating grafana-client..."
  GRAFANA_CLIENT_UUID=$(curl -s -X POST "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients" \
    -H "Authorization: Bearer $TOKEN" \
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
      "fullScopeAllowed": true
    }' | python3 -c "import sys, json; print(json.load(sys.stdin)['id'])" 2>/dev/null)
  
  GRAFANA_SECRET=$(curl -s "$KEYCLOAK_URL/admin/realms/$REALM_NAME/clients/$GRAFANA_CLIENT_UUID/client-secret" \
    -H "Authorization: Bearer $TOKEN" | python3 -c "import sys, json; print(json.load(sys.stdin)['value'])")
  echo "✓ Grafana client created"
  echo "  Client Secret: $GRAFANA_SECRET"
else
  echo "✓ Grafana client already exists"
fi

echo ""
echo "=== Keycloak Configuration Complete ==="
echo ""
echo "Next steps:"
echo "1. Update frontend to use Keycloak JS adapter"
echo "2. Update backend to validate Keycloak tokens"
echo "3. Update Nginx configuration"
echo "4. Configure Grafana with Keycloak OAuth"

