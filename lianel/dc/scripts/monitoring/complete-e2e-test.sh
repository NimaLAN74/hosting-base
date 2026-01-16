#!/bin/bash
# Complete E2E Test - Admin Access and Energy Data
# This script performs comprehensive testing of all components

set -e

cd /root/lianel/dc

# Load environment variables
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
REALM_NAME="${REALM_NAME:-lianel}"
FRONTEND_URL="${FRONTEND_URL:-https://www.lianel.se}"

echo "=========================================="
echo "COMPLETE E2E TEST - Admin & Energy Data"
echo "=========================================="
echo ""

# ============================================
# 1. TEST ENERGY DATA
# ============================================
echo "1. TESTING ENERGY DATA"
echo "----------------------"

# Test via Airflow (if connection exists)
echo "1.1 Checking database via Airflow..."
AIRFLOW_CHECK=$(docker exec dc-airflow-worker-1 python3 -c "
from airflow.providers.postgres.hooks.postgres import PostgresHook
try:
    hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    result = hook.get_first('SELECT COUNT(*) as total, COUNT(DISTINCT country_code) as countries, COUNT(DISTINCT year) as years FROM fact_energy_annual WHERE source_system = %s', parameters=('eurostat',))
    print(f'{result[0]}|{result[1]}|{result[2]}')
except Exception as e:
    print(f'ERROR|{str(e)}')
" 2>/dev/null || echo "ERROR|Connection not found")

if [[ "$AIRFLOW_CHECK" == ERROR* ]]; then
    echo "⚠️  Cannot check via Airflow: $(echo $AIRFLOW_CHECK | cut -d'|' -f2)"
else
    TOTAL=$(echo "$AIRFLOW_CHECK" | cut -d'|' -f1)
    COUNTRIES=$(echo "$AIRFLOW_CHECK" | cut -d'|' -f2)
    YEARS=$(echo "$AIRFLOW_CHECK" | cut -d'|' -f3)
    echo "✅ Database: $TOTAL records, $COUNTRIES countries, $YEARS years"
    
    if [ "$TOTAL" -lt 100 ]; then
        echo "⚠️  WARNING: Only $TOTAL records found. Expected more."
    fi
fi

# Test Energy API
echo ""
echo "1.2 Testing Energy API..."
API_INFO=$(curl -s "${FRONTEND_URL}/api/energy/info" 2>&1)
API_TOTAL=$(echo "$API_INFO" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(data.get('total_records', 0))
except:
    print('ERROR')
" 2>/dev/null)

if [ "$API_TOTAL" != "ERROR" ] && [ -n "$API_TOTAL" ]; then
    echo "✅ Energy API /info: $API_TOTAL total records"
    if [ "$API_TOTAL" -lt 100 ]; then
        echo "⚠️  WARNING: API reports only $API_TOTAL records"
    fi
else
    echo "❌ Failed to get Energy API info"
    echo "Response: $(echo "$API_INFO" | head -5)"
fi

# Test Energy API annual endpoint
echo ""
echo "1.3 Testing Energy API /annual endpoint..."
API_ANNUAL=$(curl -s "${FRONTEND_URL}/api/energy/annual?limit=10" 2>&1)
API_ANNUAL_TOTAL=$(echo "$API_ANNUAL" | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    print(data.get('total', 0))
except:
    print('ERROR')
" 2>/dev/null)

if [ "$API_ANNUAL_TOTAL" != "ERROR" ] && [ -n "$API_ANNUAL_TOTAL" ]; then
    echo "✅ Energy API /annual: $API_ANNUAL_TOTAL total records"
    if [ "$API_ANNUAL_TOTAL" -lt 100 ]; then
        echo "⚠️  WARNING: API reports only $API_ANNUAL_TOTAL records"
    fi
else
    echo "❌ Failed to get Energy API annual data"
    echo "Response: $(echo "$API_ANNUAL" | head -5)"
fi

# ============================================
# 2. TEST KEYCLOAK CONFIGURATION
# ============================================
echo ""
echo "2. TESTING KEYCLOAK CONFIGURATION"
echo "----------------------------------"

# Get admin token
echo "2.1 Getting Keycloak admin token..."
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

# Check frontend-client
echo ""
echo "2.2 Checking frontend-client configuration..."
CLIENT_UUID=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients?clientId=frontend-client" \
  -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; clients = json.load(sys.stdin); print(clients[0]['id']) if clients else exit(1)" 2>/dev/null)

if [ -z "$CLIENT_UUID" ]; then
    echo "❌ frontend-client not found"
    exit 1
fi
echo "✅ frontend-client found: ${CLIENT_UUID}"

# Check roles scope assignment
echo ""
echo "2.3 Checking roles scope assignment..."
DEFAULT_SCOPES=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients/${CLIENT_UUID}/default-client-scopes" \
  -H "Authorization: Bearer ${TOKEN}")

ROLES_SCOPE_ID="f73153cb-81df-4d64-8da2-d806ddf8bbfc"
HAS_ROLES_SCOPE=$(echo "$DEFAULT_SCOPES" | python3 -c "import sys, json; scopes = json.load(sys.stdin); print('yes' if any(s['id'] == '${ROLES_SCOPE_ID}') for s in scopes) else 'no'" 2>/dev/null)

if [ "$HAS_ROLES_SCOPE" = "yes" ]; then
    echo "✅ roles scope is assigned to frontend-client"
else
    echo "❌ roles scope NOT assigned to frontend-client"
    echo "   Attempting to assign..."
    HTTP_CODE=$(curl -s -w "\n%{http_code}" -X PUT "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/clients/${CLIENT_UUID}/default-client-scopes/${ROLES_SCOPE_ID}" \
      -H "Authorization: Bearer ${TOKEN}" \
      -H "Content-Type: application/json" 2>&1 | tail -1)
    
    if [ "$HTTP_CODE" = "204" ] || [ "$HTTP_CODE" = "200" ]; then
        echo "✅ roles scope assigned successfully"
    else
        echo "❌ Failed to assign roles scope. HTTP code: ${HTTP_CODE}"
    fi
fi

# Check admin role assignment
echo ""
echo "2.4 Checking admin role assignment..."
USERS=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/users" \
  -H "Authorization: Bearer ${TOKEN}")

USER_COUNT=$(echo "$USERS" | python3 -c "import sys, json; users = json.load(sys.stdin); print(len(users))" 2>/dev/null)
echo "Found $USER_COUNT users in realm"

# Check each user for admin role
echo "$USERS" | python3 -c "
import sys, json
users = json.load(sys.stdin)
for user in users:
    username = user.get('username', 'N/A')
    user_id = user.get('id', '')
    print(f'Checking user: {username}')
" 2>/dev/null

# Check admin user specifically
ADMIN_USER_ID=$(echo "$USERS" | python3 -c "import sys, json; users = json.load(sys.stdin); [print(u['id']) for u in users if u.get('username') == 'admin']" 2>/dev/null)

if [ -n "$ADMIN_USER_ID" ]; then
    ADMIN_ROLES=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/users/${ADMIN_USER_ID}/role-mappings/realm" \
      -H "Authorization: Bearer ${TOKEN}")
    
    HAS_ADMIN_ROLE=$(echo "$ADMIN_ROLES" | python3 -c "import sys, json; roles = json.load(sys.stdin); print('yes' if roles and any(r['name'].lower() in ['admin', 'realm-admin'] for r in roles) else 'no')" 2>/dev/null)
    
    if [ "$HAS_ADMIN_ROLE" = "yes" ]; then
        echo "✅ Admin user has admin role assigned"
    else
        echo "❌ Admin user does NOT have admin role"
        echo "   Attempting to assign admin role..."
        
        # Get admin role ID
        ADMIN_ROLE_ID=$(curl -s "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/roles/admin" \
          -H "Authorization: Bearer ${TOKEN}" | python3 -c "import sys, json; print(json.load(sys.stdin).get('id', ''))" 2>/dev/null)
        
        if [ -n "$ADMIN_ROLE_ID" ]; then
            ASSIGN_RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "${KEYCLOAK_URL}/admin/realms/${REALM_NAME}/users/${ADMIN_USER_ID}/role-mappings/realm" \
              -H "Authorization: Bearer ${TOKEN}" \
              -H "Content-Type: application/json" \
              -d "[{\"id\":\"${ADMIN_ROLE_ID}\",\"name\":\"admin\"}]" 2>&1)
            
            HTTP_CODE=$(echo "$ASSIGN_RESPONSE" | tail -1)
            if [ "$HTTP_CODE" = "204" ] || [ "$HTTP_CODE" = "200" ]; then
                echo "✅ Admin role assigned successfully"
            else
                echo "❌ Failed to assign admin role. HTTP code: ${HTTP_CODE}"
            fi
        else
            echo "❌ Admin role not found in realm"
        fi
    fi
else
    echo "⚠️  Admin user not found"
fi

# ============================================
# 3. TEST BACKEND ADMIN API
# ============================================
echo ""
echo "3. TESTING BACKEND ADMIN API"
echo "----------------------------"

# Test without auth (should return 401)
echo "3.1 Testing admin check endpoint without auth..."
ADMIN_CHECK_NO_AUTH=$(curl -s -w "\n%{http_code}" "${FRONTEND_URL}/api/admin/check" 2>&1 | tail -1)
if [ "$ADMIN_CHECK_NO_AUTH" = "401" ] || [ "$ADMIN_CHECK_NO_AUTH" = "403" ]; then
    echo "✅ Admin check endpoint accessible (returns $ADMIN_CHECK_NO_AUTH without auth)"
else
    echo "⚠️  Admin check endpoint returned: $ADMIN_CHECK_NO_AUTH"
fi

# Test with a user token (if we can get one)
echo ""
echo "3.2 Testing admin check with user token..."
# Get a user token for admin user
USER_TOKEN=$(curl -s -X POST "${KEYCLOAK_URL}/realms/${REALM_NAME}/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=admin" \
  -d "password=${KEYCLOAK_ADMIN_PASSWORD}" \
  -d "grant_type=password" \
  -d "client_id=frontend-client" | python3 -c "import sys, json; print(json.load(sys.stdin).get('access_token', ''))" 2>/dev/null)

if [ -n "$USER_TOKEN" ]; then
    ADMIN_CHECK_WITH_AUTH=$(curl -s -w "\n%{http_code}" "${FRONTEND_URL}/api/admin/check" \
      -H "Authorization: Bearer ${USER_TOKEN}" 2>&1)
    
    HTTP_CODE=$(echo "$ADMIN_CHECK_WITH_AUTH" | tail -1)
    BODY=$(echo "$ADMIN_CHECK_WITH_AUTH" | head -n-1)
    
    if [ "$HTTP_CODE" = "200" ]; then
        IS_ADMIN=$(echo "$BODY" | python3 -c "import sys, json; data = json.load(sys.stdin); print(data.get('isAdmin', False))" 2>/dev/null)
        if [ "$IS_ADMIN" = "True" ]; then
            echo "✅ Admin check API returns isAdmin: true"
        else
            echo "❌ Admin check API returns isAdmin: false (user should be admin)"
        fi
    else
        echo "⚠️  Admin check API returned HTTP $HTTP_CODE"
        echo "Response: $BODY"
    fi
    
    # Decode token to check roles
    echo ""
    echo "3.3 Decoding user token to check roles..."
    TOKEN_PARTS=$(echo "$USER_TOKEN" | cut -d'.' -f2)
    # Add padding if needed
    PADDED=$(echo "$TOKEN_PARTS" | sed 's/-/+/g; s/_/\//g')
    PADDING=$((4 - ${#PADDED} % 4))
    if [ $PADDING -ne 4 ]; then
        PADDED="${PADDED}$(printf '%*s' $PADDING | tr ' ' '=')"
    fi
    
    TOKEN_JSON=$(echo "$PADDED" | base64 -d 2>/dev/null | python3 -c "
import sys, json
try:
    data = json.load(sys.stdin)
    realm_access = data.get('realm_access', {})
    roles = realm_access.get('roles', [])
    print(f\"Roles in token: {', '.join(roles) if roles else 'NONE'}\")
    has_admin = any(r.lower() in ['admin', 'realm-admin'] for r in roles)
    print(f\"Has admin role: {has_admin}\")
except Exception as e:
    print(f\"Error: {e}\")
" 2>/dev/null)
    
    echo "$TOKEN_JSON"
else
    echo "⚠️  Could not get user token for testing"
fi

# ============================================
# 4. SUMMARY
# ============================================
echo ""
echo "=========================================="
echo "TEST SUMMARY"
echo "=========================================="
echo ""
echo "Energy Data:"
if [ "$API_TOTAL" != "ERROR" ] && [ "$API_TOTAL" -ge 100 ]; then
    echo "  ✅ Energy data available ($API_TOTAL records)"
else
    echo "  ❌ Energy data issue: Only $API_TOTAL records found"
    echo "     Action: Wait for harmonization DAG to complete or check ingestion DAGs"
fi

echo ""
echo "Keycloak Configuration:"
if [ "$HAS_ROLES_SCOPE" = "yes" ]; then
    echo "  ✅ Roles scope assigned to frontend-client"
else
    echo "  ❌ Roles scope NOT assigned"
fi

if [ "$HAS_ADMIN_ROLE" = "yes" ]; then
    echo "  ✅ Admin user has admin role"
else
    echo "  ❌ Admin user missing admin role"
fi

echo ""
echo "Next Steps:"
echo "1. If roles scope was just assigned: User must LOG OUT and LOG BACK IN"
echo "2. If admin role was just assigned: User must LOG OUT and LOG BACK IN"
echo "3. Check browser console for: === DASHBOARD ADMIN ROLE DEBUG ==="
echo "4. Verify admin services are visible in frontend"
echo "5. For energy data: Check harmonization DAG status in Airflow"
