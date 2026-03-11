#!/usr/bin/env bash
# Server-side E2E: logs + login flow (GET+POST with cookie) + watchlist + summary.
# Run ON the server (e.g. in /root/hosting-base or /root/lianel/dc): bash scripts/monitoring/e2e-server-full-analysis.sh
# Requires: curl, docker. Optional: jq.
set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'
FAILED=0

echo "=============================================="
echo "E2E Server: Logs + Login + Watchlist"
echo "=============================================="

# --- 1. LOGS (last 15 lines each) ---
echo ""
echo "========== 1. CONTAINER LOGS (recent) =========="
echo "--- oauth2-proxy (last 15) ---"
docker logs oauth2-proxy --tail 15 2>&1 | tail -15
echo ""
echo "--- keycloak (last 15) ---"
docker logs keycloak --tail 15 2>&1 | tail -15
echo ""
echo "--- grafana (last 10) ---"
docker logs grafana --tail 10 2>&1 | tail -10
echo ""
echo "--- stock-service (last 10, watchlist/IBKR only) ---"
docker logs lianel-stock-service --tail 30 2>&1 | grep -E "watchlist|IBKR|tickle|snapshot|WARN|error" || echo "(no matching lines)"

# --- 2. LOGIN FLOW: GET auth URL, save cookie, POST authenticate ---
echo ""
echo "========== 2. LOGIN FLOW (cookie + POST) =========="
COOKIE_JAR=$(mktemp)
trap "rm -f $COOKIE_JAR" EXIT
AUTH_URL="https://www.lianel.se/auth/realms/lianel/protocol/openid-connect/auth?client_id=grafana-client&redirect_uri=https%3A%2F%2Fmonitoring.lianel.se%2Flogin%2Fgeneric_oauth&response_type=code&scope=openid"

echo "2.1 GET login page (save cookies)..."
GET_CODE=$(curl -s -k -o /tmp/e2e_login.html -w "%{http_code}" -c "$COOKIE_JAR" -b "$COOKIE_JAR" "$AUTH_URL" -L)
echo "    HTTP $GET_CODE"
if [ "$GET_CODE" != "200" ]; then
  echo -e "    ${RED}FAIL: expected 200${NC}"
  FAILED=1
else
  echo -e "    ${GREEN}OK${NC}"
fi

echo "2.2 Extract form action..."
ACTION=$(grep -oE 'action="[^"]+"' /tmp/e2e_login.html | head -1 | sed 's/action="//;s/"//;s/&amp;/\&/g')
if [ -z "$ACTION" ]; then
  echo -e "    ${YELLOW}WARN: no form action found${NC}"
else
  echo "    action: ${ACTION:0:80}..."
fi

echo "2.3 POST login (with saved cookie; expect 200 and login page again if bad creds, or redirect if success)..."
POST_CODE=$(curl -s -k -o /tmp/e2e_post.html -w "%{http_code}" -c "$COOKIE_JAR" -b "$COOKIE_JAR" -X POST "$ACTION" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  --data-raw "username=testuser&password=testpass&login=Log+in")
echo "    HTTP $POST_CODE"
if [ "$POST_CODE" = "200" ]; then
  if grep -q "cookie_not_found\|Invalid username or password" /tmp/e2e_post.html 2>/dev/null; then
    echo -e "    ${YELLOW}Body contains error (expected if creds wrong); if cookie_not_found then cookie path is still broken${NC}"
  else
    echo -e "    ${GREEN}200 + form (cookie was sent; login failed only due to wrong creds)${NC}"
  fi
elif [ "$POST_CODE" = "302" ]; then
  echo -e "    ${GREEN}OK: redirect (login may have succeeded)${NC}"
else
  echo -e "    ${YELLOW}HTTP $POST_CODE${NC}"
fi

# --- 3. WATCHLIST API ---
echo ""
echo "========== 3. WATCHLIST API =========="
WATCH_CODE=$(curl -s -k -o /tmp/e2e_watchlist.json -w "%{http_code}" "https://www.lianel.se/api/v1/stock-service/watchlist" -H "Host: www.lianel.se")
echo "3.1 GET /api/v1/stock-service/watchlist: HTTP $WATCH_CODE"
if [ "$WATCH_CODE" != "200" ]; then
  echo -e "    ${RED}FAIL: backend or proxy returned $WATCH_CODE${NC}"
  FAILED=1
else
  if command -v jq >/dev/null 2>&1; then
    FIRST_ERR=$(jq -r '.symbols[0].error // "none"' /tmp/e2e_watchlist.json 2>/dev/null)
    FIRST_PRICE=$(jq -r '.symbols[0].price // "null"' /tmp/e2e_watchlist.json 2>/dev/null)
  else
    FIRST_ERR=$(grep -o '"error":"[^"]*"' /tmp/e2e_watchlist.json 2>/dev/null | head -1 | sed 's/"error":"//;s/"$//')
    FIRST_PRICE="(check JSON)"
  fi
  echo "    First symbol: price=$FIRST_PRICE, error=${FIRST_ERR:-none}"
  if [ -n "$FIRST_ERR" ] && [ "$FIRST_ERR" != "none" ]; then
    echo -e "    ${YELLOW}Backend returns data; prices null because provider error (e.g. IBKR 'no bridge' = run Client Portal Gateway)${NC}"
  else
    echo -e "    ${GREEN}OK: Watchlist has prices${NC}"
  fi
fi

# --- 4. GRAFANA CONFIG (auth_url must be www.lianel.se/auth for cookie path) ---
echo ""
echo "========== 4. GRAFANA AUTH URL (must be www for cookie) =========="
GRAFANA_AUTH=$(docker exec grafana grep '^auth_url' /etc/grafana/grafana.ini 2>/dev/null || true)
echo "$GRAFANA_AUTH"
if echo "$GRAFANA_AUTH" | grep -q 'www.lianel.se/auth'; then
  echo -e "    ${GREEN}OK: Login flow on www → cookie path correct${NC}"
else
  echo -e "    ${RED}FAIL: Use auth_url = https://www.lianel.se/auth/realms/... (and restart Grafana)${NC}"
  FAILED=1
fi

echo ""
echo "=============================================="
if [ $FAILED -eq 1 ]; then
  echo -e "${RED}E2E Server: FAILED${NC}"
  exit 1
fi
echo -e "${GREEN}E2E Server: PASSED${NC}"
exit 0
