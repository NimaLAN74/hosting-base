#!/bin/bash
# End-to-End Test: Admin Access and Energy Data
# This script tests:
# 1. Energy data count and availability
# 2. Admin role detection via backend API
# 3. Frontend admin services visibility

set -e

echo "=== E2E Test: Admin Access and Energy Data ==="
echo ""

# Load environment variables
if [ -f .env ]; then
    export $(cat .env | grep -v '^#' | xargs)
fi

KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
REALM_NAME="${REALM_NAME:-lianel}"
FRONTEND_URL="${FRONTEND_URL:-https://www.lianel.se}"

echo "1. Testing Energy Data Availability"
echo "-----------------------------------"

# Check database via Airflow
echo "Checking database records..."
DB_RESULT=$(docker exec dc-airflow-worker-1 python3 -c "
from airflow.providers.postgres.hooks.postgres import PostgresHook
hook = PostgresHook(postgres_conn_id='lianel_energy_db')
result = hook.get_first('SELECT COUNT(*) as total, COUNT(DISTINCT country_code) as countries, COUNT(DISTINCT year) as years FROM fact_energy_annual WHERE source_system = %s', parameters=('eurostat',))
print(f'{result[0]}|{result[1]}|{result[2]}')
" 2>/dev/null)

if [ -n "$DB_RESULT" ]; then
    TOTAL=$(echo "$DB_RESULT" | cut -d'|' -f1)
    COUNTRIES=$(echo "$DB_RESULT" | cut -d'|' -f2)
    YEARS=$(echo "$DB_RESULT" | cut -d'|' -f3)
    echo "✅ Database: $TOTAL records, $COUNTRIES countries, $YEARS years"
    
    if [ "$TOTAL" -lt 100 ]; then
        echo "⚠️  WARNING: Only $TOTAL records found. Expected more after ingestion."
    fi
else
    echo "❌ Failed to query database"
fi

# Check API endpoint
echo ""
echo "Testing Energy API endpoint..."
API_RESULT=$(curl -s "${FRONTEND_URL}/api/energy/info" 2>&1 | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    total = data.get('total_records', 0)
    print(f'{total}')
except:
    print('ERROR')
" 2>/dev/null)

if [ "$API_RESULT" != "ERROR" ] && [ -n "$API_RESULT" ]; then
    echo "✅ Energy API: $API_RESULT total records"
    if [ "$API_RESULT" -lt 100 ]; then
        echo "⚠️  WARNING: API reports only $API_RESULT records"
    fi
else
    echo "❌ Failed to query Energy API"
fi

echo ""
echo "2. Testing Admin Role Configuration"
echo "-----------------------------------"

# Get admin token
echo "Getting Keycloak admin token..."
TOKEN=$(curl -s -X POST "${KEYCLOAK_URL}/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=${KEYCLOAK_ADMIN_USER}" \
  -d "password=${KEYCLOAK_ADMIN_PASSWORD}" \
  -d "grant_type=password" \
  -d "client_id=admin-cli" | python3 -c "import sys, json; print(json.load(sys.stdin).get('access_token', ''))" 2>/dev/null)

if [ -z "$TOKEN" ]; then
    echo "❌ Failed to get admin token"
    exit 1
fi

echo "✅ Admin token obtained"

# Check frontend-client scope
echo ""
echo "Checking frontend-client scope configuration..."
CLIENT_UUID=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients?clientId=frontend-client" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; clients = json.load(sys.stdin); print(clients[0]['id']) if clients else exit(1)" 2>/dev/null)

if [ -z "$CLIENT_UUID" ]; then
    echo "❌ frontend-client not found"
    exit 1
fi

REALM_ROLES_SCOPE_ID=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/client-scopes" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; scopes = json.load(sys.stdin); [print(s['id']) for s in scopes if s['name'] == 'realm-roles']" 2>/dev/null)

if [ -z "$REALM_ROLES_SCOPE_ID" ]; then
    echo "❌ realm-roles scope not found"
    exit 1
fi

DEFAULT_SCOPES=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients/${CLIENT_UUID}/default-client-scopes" \
  -H "Authorization: Bearer ${TOKEN}")

HAS_REALM_ROLES=$(echo "$DEFAULT_SCOPES" | python3 -c "import sys, json; scopes = json.load(sys.stdin); print('yes' if any(s['id'] == '${REALM_ROLES_SCOPE_ID}') for s in scopes) else 'no'" 2>/dev/null)

if [ "$HAS_REALM_ROLES" = "yes" ]; then
    echo "✅ realm-roles scope is assigned to frontend-client"
else
    echo "❌ realm-roles scope NOT assigned to frontend-client"
    echo "   Run: bash fix-keycloak-client-scope-realm-roles.sh"
fi

# Check admin role assignment
echo ""
echo "Checking admin role assignment..."
echo "Please provide username to check (or press Enter to skip):"
read -r USERNAME

if [ -n "$USERNAME" ]; then
    USER_ID=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/users?username=${USERNAME}" \
      -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; users = json.load(sys.stdin); print(users[0]['id']) if users else exit(1)" 2>/dev/null)
    
    if [ -n "$USER_ID" ]; then
        USER_ROLES=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/users/${USER_ID}/role-mappings/realm" \
          -H "Authorization: Bearer ${TOKEN}")
        
        HAS_ADMIN=$(echo "$USER_ROLES" | python3 -c "import sys, json; roles = json.load(sys.stdin); print('yes' if roles and any(r['name'].lower() in ['admin', 'realm-admin'] for r in roles) else 'no')" 2>/dev/null)
        
        if [ "$HAS_ADMIN" = "yes" ]; then
            echo "✅ User $USERNAME has admin role assigned"
        else
            echo "❌ User $USERNAME does NOT have admin role"
            echo "   Run: bash assign-admin-role.sh $USERNAME"
        fi
    else
        echo "❌ User $USERNAME not found"
    fi
fi

echo ""
echo "3. Testing Backend Admin API"
echo "-----------------------------"

# Test admin check endpoint (will fail without auth, but tests endpoint exists)
ADMIN_CHECK=$(curl -s -w "\n%{http_code}" "${FRONTEND_URL}/api/admin/check" 2>&1 | tail -1)
if [ "$ADMIN_CHECK" = "401" ] || [ "$ADMIN_CHECK" = "403" ]; then
    echo "✅ Admin check endpoint is accessible (returns $ADMIN_CHECK without auth)"
else
    echo "⚠️  Admin check endpoint returned: $ADMIN_CHECK"
fi

echo ""
echo "=== Test Summary ==="
echo ""
echo "Next Steps:"
echo "1. If realm-roles scope is not assigned, run:"
echo "   bash fix-keycloak-client-scope-realm-roles.sh"
echo ""
echo "2. If user doesn't have admin role, run:"
echo "   bash assign-admin-role.sh <username>"
echo ""
echo "3. User must LOG OUT and LOG BACK IN after role changes"
echo ""
echo "4. Check browser console for debug output:"
echo "   === DASHBOARD ADMIN ROLE DEBUG ==="
echo ""
echo "5. Verify frontend shows admin services when logged in as admin"
