#!/bin/bash
# E2E Stock Service: IBKR (via backend), backend APIs, frontend. Run from CI or repo root.
# 1) Backend health + status (public)
# 2) Backend /ibkr/verify without auth -> 401
# 3) Optional: /ibkr/verify with Keycloak token (tests full IBKR path; 200 or 401/502 from IBKR both mean endpoint works)
# 4) Frontend /stock/ and /stock-app redirect
set -e

BASE_URL="${BASE_URL:-https://www.lianel.se}"
KEYCLOAK_URL="${KEYCLOAK_URL:-${BASE_URL}/auth}"
KEYCLOAK_REALM="${KEYCLOAK_REALM:-lianel}"
FAILED=0

# Optional: get Bearer token for protected endpoints (client_credentials or password)
# Set E2E_STOCK_KEYCLOAK_CLIENT_ID + E2E_STOCK_KEYCLOAK_CLIENT_SECRET, or
# E2E_STOCK_KEYCLOAK_USER + E2E_STOCK_KEYCLOAK_PASSWORD (with optional E2E_STOCK_KEYCLOAK_CLIENT_ID)
get_token() {
  if [ -n "$E2E_STOCK_KEYCLOAK_CLIENT_ID" ] && [ -n "$E2E_STOCK_KEYCLOAK_CLIENT_SECRET" ]; then
    BODY="client_id=${E2E_STOCK_KEYCLOAK_CLIENT_ID}&client_secret=${E2E_STOCK_KEYCLOAK_CLIENT_SECRET}&grant_type=client_credentials"
    RESP=$(curl -sS -w "\n%{http_code}" -X POST "${KEYCLOAK_URL}/realms/${KEYCLOAK_REALM}/protocol/openid-connect/token" \
      -H "Content-Type: application/x-www-form-urlencoded" -d "$BODY")
    CODE=$(echo "$RESP" | tail -1)
    BOD=$(echo "$RESP" | sed '$d')
    [ "$CODE" = "200" ] && echo "$BOD" | jq -r '.access_token // empty'
    return 0
  fi
  if [ -n "$E2E_STOCK_KEYCLOAK_USER" ] && [ -n "$E2E_STOCK_KEYCLOAK_PASSWORD" ]; then
    CID="${E2E_STOCK_KEYCLOAK_CLIENT_ID:-frontend-client}"
    BODY="client_id=${CID}&username=${E2E_STOCK_KEYCLOAK_USER}&password=${E2E_STOCK_KEYCLOAK_PASSWORD}&grant_type=password"
    [ -n "$E2E_STOCK_KEYCLOAK_CLIENT_SECRET" ] && BODY="${BODY}&client_secret=${E2E_STOCK_KEYCLOAK_CLIENT_SECRET}"
    RESP=$(curl -sS -w "\n%{http_code}" -X POST "${KEYCLOAK_URL}/realms/${KEYCLOAK_REALM}/protocol/openid-connect/token" \
      -H "Content-Type: application/x-www-form-urlencoded" -d "$BODY")
    CODE=$(echo "$RESP" | tail -1)
    BOD=$(echo "$RESP" | sed '$d')
    [ "$CODE" = "200" ] && echo "$BOD" | jq -r '.access_token // empty'
    return 0
  fi
  return 1
}

echo "=== E2E Stock Service (IBKR → Backend → Frontend) ==="
echo "BASE_URL=$BASE_URL KEYCLOAK_URL=$KEYCLOAK_URL KEYCLOAK_REALM=$KEYCLOAK_REALM"
echo ""

# --- Backend (public) ---
echo "--- Backend (public) ---"
echo "1. GET /api/v1/stock-service/health (expect 200)..."
HEALTH_CODE=$(curl -sS -o /dev/null -w "%{http_code}" "$BASE_URL/api/v1/stock-service/health" 2>/dev/null)
if [ "$HEALTH_CODE" = "200" ]; then
  echo "   ✅ Stock API health: 200"
else
  echo "   ❌ Stock API health: HTTP $HEALTH_CODE"
  FAILED=1
fi

echo "2. GET /api/v1/stock-service/status (expect 200, JSON)..."
STATUS_RESP=$(curl -sS -w "\n%{http_code}" "$BASE_URL/api/v1/stock-service/status" 2>/dev/null)
STATUS_CODE=$(echo "$STATUS_RESP" | tail -1)
STATUS_BODY=$(echo "$STATUS_RESP" | sed '$d')
if [ "$STATUS_CODE" = "200" ]; then
  if command -v jq >/dev/null 2>&1 && echo "$STATUS_BODY" | jq -e . >/dev/null 2>&1; then
    echo "   ✅ Stock API status: 200, valid JSON"
  elif echo "$STATUS_BODY" | grep -qE '"service"|"ibkr_oauth_configured"'; then
    echo "   ✅ Stock API status: 200, JSON (service/ibkr present)"
  else
    echo "   ❌ Stock API status: 200 but invalid or unexpected JSON"
    FAILED=1
  fi
else
  echo "   ❌ Stock API status: HTTP $STATUS_CODE"
  FAILED=1
fi
echo ""

# --- Backend protected: ibkr/verify without auth -> 401 ---
echo "--- Backend (protected) ---"
echo "3. GET /api/v1/stock-service/ibkr/verify without auth (expect 401)..."
VERIFY_NOAUTH=$(curl -sS -w "\n%{http_code}" "$BASE_URL/api/v1/stock-service/ibkr/verify" 2>/dev/null)
VERIFY_NOAUTH_CODE=$(echo "$VERIFY_NOAUTH" | tail -1)
if [ "$VERIFY_NOAUTH_CODE" = "401" ]; then
  echo "   ✅ ibkr/verify without auth: 401 (protected)"
else
  echo "   ❌ ibkr/verify without auth: HTTP $VERIFY_NOAUTH_CODE (expected 401)"
  FAILED=1
fi
echo ""

# --- Optional: IBKR verify with token (full E2E) ---
echo "--- IBKR verify (with auth, optional) ---"
TOKEN=$(get_token) || true
if [ -n "$TOKEN" ]; then
  echo "4. GET /api/v1/stock-service/ibkr/verify with Bearer (IBKR call)..."
  VERIFY_RESP=$(curl -sS -w "\n%{http_code}" -H "Authorization: Bearer $TOKEN" "$BASE_URL/api/v1/stock-service/ibkr/verify" 2>/dev/null)
  VERIFY_CODE=$(echo "$VERIFY_RESP" | tail -1)
  VERIFY_BODY=$(echo "$VERIFY_RESP" | sed '$d')
  if [ "$VERIFY_CODE" = "200" ]; then
    echo "   ✅ ibkr/verify: 200 (IBKR authentication OK)"
  elif [ "$VERIFY_CODE" = "401" ]; then
    echo "   ⚠️  ibkr/verify: 401 (IBKR invalid consumer or auth – check server IBKR OAuth config)"
    echo "      Body: ${VERIFY_BODY:0:120}..."
  elif [ "$VERIFY_CODE" = "502" ]; then
    echo "   ⚠️  ibkr/verify: 502 (IBKR error or unreachable – check server IBKR config)"
    echo "      Body: ${VERIFY_BODY:0:120}..."
  else
    echo "   ❌ ibkr/verify: HTTP $VERIFY_CODE (unexpected)"
    echo "      Body: ${VERIFY_BODY:0:200}"
    FAILED=1
  fi
else
  echo "4. (skip) No E2E Keycloak credentials – set E2E_STOCK_KEYCLOAK_* to test ibkr/verify with auth"
fi
echo ""

# --- Frontend ---
echo "--- Frontend ---"
echo "5. GET /stock-app (expect 302 to /stock/)..."
REDIR_OUT=$(curl -sS -o /dev/null -w "%{http_code}" -D - "$BASE_URL/stock-app" 2>/dev/null)
CODE=$(echo "$REDIR_OUT" | tail -1)
LOCATION=$(echo "$REDIR_OUT" | grep -i '^Location:' | head -1 | tr -d '\r' | cut -d' ' -f2-)
if [ "$CODE" = "302" ] && echo "$LOCATION" | grep -q '/stock'; then
  echo "   ✅ /stock-app -> 302 Location: $LOCATION"
elif [ "$CODE" = "200" ]; then
  echo "   ⚠️  /stock-app returned 200 (expected 302)"
  FAILED=1
else
  echo "   ❌ /stock-app HTTP $CODE, Location: $LOCATION"
  FAILED=1
fi

echo "6. GET /stock/ (expect 200, main app HTML with Stock Service / IBKR)..."
STOCK_BODY=$(curl -sS "$BASE_URL/stock/" 2>/dev/null)
STOCK_CODE=$(curl -sS -o /dev/null -w "%{http_code}" "$BASE_URL/stock/" 2>/dev/null)
if [ "$STOCK_CODE" = "200" ] && (echo "$STOCK_BODY" | grep -qE 'Lianel World|root|static/js'); then
  if echo "$STOCK_BODY" | grep -qE 'Stock Service|IBKR|stock'; then
    echo "   ✅ /stock/ -> 200, main app HTML (Stock/IBKR content present)"
  else
    echo "   ✅ /stock/ -> 200, main app HTML (Stock/IBKR text not in initial HTML; may load via JS)"
  fi
else
  echo "   ❌ /stock/ HTTP $STOCK_CODE or wrong body"
  FAILED=1
fi
echo ""

if [ $FAILED -eq 1 ]; then
  echo "=== E2E Stock Service: FAILED ==="
  exit 1
fi
echo "=== E2E Stock Service: PASSED ==="
exit 0
