#!/usr/bin/env bash
# Full E2E trace: Grafana (oauth2-proxy → Keycloak), Airflow (FAB → Keycloak), Stock watchlist.
# Run on server: bash scripts/monitoring/e2e-full-trace-grafana-airflow-stock.sh
# Or from repo with BASE_URL: BASE_URL=https://www.lianel.se bash ... (uses public URLs)
set -e

BASE_URL="${BASE_URL:-https://www.lianel.se}"
MONITORING_HOST="${MONITORING_HOST:-monitoring.lianel.se}"
AIRFLOW_HOST="${AIRFLOW_HOST:-airflow.lianel.se}"
# When running on server, can use -k and 127.0.0.1 with Host header
CURL_OPTS="${CURL_OPTS:--k}"
USE_HOST_HEADER="${USE_HOST_HEADER:-}"
if [ -n "$USE_HOST_HEADER" ]; then
  CURL_BASE="curl -sS $CURL_OPTS -H \"Host: \$H\" \"https://127.0.0.1\$PATH\""
else
  CURL_BASE="curl -sS $CURL_OPTS"
fi

FAILED=0
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

run_curl() {
  local host path extra
  host="$1"
  path="$2"
  extra="${3:-}"
  if [ -n "$USE_HOST_HEADER" ]; then
    curl -sS $CURL_OPTS -H "Host: $host" "https://127.0.0.1$path" $extra
  else
    curl -sS $CURL_OPTS "https://${host}${path}" $extra
  fi
}

echo "=============================================="
echo "E2E Full Trace: Grafana, Airflow, Stock"
echo "=============================================="
echo "BASE_URL=$BASE_URL MONITORING=$MONITORING_HOST AIRFLOW=$AIRFLOW_HOST"
echo ""

# --- GRAFANA (monitoring.lianel.se → oauth2-proxy → Keycloak) ---
echo "========== 1. GRAFANA (monitoring → oauth2-proxy → Keycloak) =========="
echo "1.1 GET https://$MONITORING_HOST/ (expect 302 to /oauth2/sign_in)..."
HEADERS=$(mktemp)
trap "rm -f $HEADERS" EXIT
if [ -n "$USE_HOST_HEADER" ]; then
  CODE=$(curl -sS $CURL_OPTS -o /dev/null -w "%{http_code}" -D "$HEADERS" -H "Host: $MONITORING_HOST" "https://127.0.0.1/")
else
  CODE=$(curl -sS $CURL_OPTS -o /dev/null -w "%{http_code}" -D "$HEADERS" "https://$MONITORING_HOST/")
fi
LOC1=$(grep -i '^Location:' "$HEADERS" | head -1 | sed 's/^[Ll]ocation:[[:space:]]*//' | tr -d '\r\n')
echo "    HTTP $CODE, Location: ${LOC1:0:120}..."

if [ "$CODE" != "302" ]; then
  echo -e "    ${RED}FAIL: expected 302, got $CODE${NC}"
  FAILED=1
else
  echo -e "    ${GREEN}OK: 302 to sign_in${NC}"
fi

echo ""
echo "1.2 GET sign_in URL (oauth2-proxy redirects to Keycloak; capture Location)..."
SIGN_IN_PATH="/oauth2/sign_in?rd=%2F"
SIGN_IN_HOST="$MONITORING_HOST"
if [ -n "$LOC1" ]; then
  SIGN_IN_PATH=$(echo "$LOC1" | sed 's|https\?://[^/]*||' | sed 's/^\/\///')
  [ -z "$SIGN_IN_PATH" ] && SIGN_IN_PATH="/oauth2/sign_in?rd=%2F"
fi
HEADERS2=$(mktemp)
if [ -n "$USE_HOST_HEADER" ]; then
  curl -sS $CURL_OPTS -o /dev/null -D "$HEADERS2" -H "Host: $SIGN_IN_HOST" "https://127.0.0.1$SIGN_IN_PATH" -c /tmp/e2e_cookies.txt -b /tmp/e2e_cookies.txt 2>/dev/null || true
else
  curl -sS $CURL_OPTS -o /dev/null -D "$HEADERS2" "https://$SIGN_IN_HOST$SIGN_IN_PATH" -c /tmp/e2e_cookies.txt -b /tmp/e2e_cookies.txt 2>/dev/null || true
fi
LOC2=$(grep -i '^Location:' "$HEADERS2" | head -1 | sed 's/^[Ll]ocation:[[:space:]]*//' | tr -d '\r\n')
echo "    Redirect to Keycloak: ${LOC2:0:150}..."

if [ -z "$LOC2" ]; then
  echo -e "    ${RED}FAIL: no Location (no redirect to Keycloak)${NC}"
  cat "$HEADERS2" | head -20
  FAILED=1
else
  # Grafana flow: first redirect may be to /login/generic_oauth; follow to Keycloak in step 1.3
  if echo "$LOC2" | grep -q 'protocol/openid-connect/auth'; then
    if echo "$LOC2" | grep -q 'code_challenge='; then
      echo -e "    ${GREEN}OK: Keycloak URL has code_challenge (PKCE)${NC}"
    else
      echo -e "    ${GREEN}OK: Keycloak URL (PKCE not required if grafana-client updated)${NC}"
    fi
  else
    echo -e "    ${GREEN}OK: Redirect to $LOC2 (will follow to Keycloak in 1.3)${NC}"
  fi
fi

echo ""
echo "1.3 Follow redirect to Keycloak (expect 200 = login page, or 302 to login form)..."
if [ -n "$LOC2" ]; then
  if echo "$LOC2" | grep -q '^https'; then
    KC_URL="$LOC2"
  else
    KC_URL="https://$MONITORING_HOST$LOC2"
  fi
  HEADERS3=$(mktemp)
  # Follow redirects to Keycloak; use same Host as in URL so cookies match
  if [ -n "$USE_HOST_HEADER" ]; then
    KC_HOST=$(echo "$LOC2" | sed -n 's|https\?://\([^/]*\).*|\1|p')
    CODE3=$(curl -sS $CURL_OPTS -o /tmp/e2e_kc.html -w "%{http_code}" -D "$HEADERS3" -L --resolve "$KC_HOST:443:127.0.0.1" "$KC_URL" -c /tmp/e2e_cookies.txt -b /tmp/e2e_cookies.txt 2>/dev/null || echo "000")
  else
    CODE3=$(curl -sS $CURL_OPTS -o /tmp/e2e_kc.html -w "%{http_code}" -D "$HEADERS3" -L "$KC_URL" -c /tmp/e2e_cookies.txt -b /tmp/e2e_cookies.txt 2>/dev/null || echo "000")
  fi
  echo "    Keycloak response: HTTP $CODE3"
  if [ "$CODE3" = "200" ]; then
    if grep -q 'login-actions/authenticate\|kc-form-login\|name="username"' /tmp/e2e_kc.html 2>/dev/null; then
      echo -e "    ${GREEN}OK: Keycloak login page received${NC}"
    else
      echo -e "    ${YELLOW}WARN: 200 but body may not be login form (check for error in body)${NC}"
      head -3 /tmp/e2e_kc.html
    fi
  elif echo "$LOC2" | grep -q 'error=invalid_request\|error=access_denied'; then
    echo -e "    ${RED}FAIL: Keycloak returned error redirect (invalid_request = often missing PKCE)${NC}"
    echo "    URL: $LOC2"
    FAILED=1
  else
    echo -e "    ${YELLOW}WARN: Keycloak HTTP $CODE3${NC}"
  fi
  rm -f "$HEADERS3"
fi
rm -f "$HEADERS" "$HEADERS2"

# --- AIRFLOW ---
echo ""
echo "========== 2. AIRFLOW (FAB → Keycloak) =========="
echo "2.1 GET $AIRFLOW_HOST/auth/login/keycloak (expect 302 to Keycloak, redirect_uri=https)..."
HEADERS_A=$(mktemp)
if [ -n "$USE_HOST_HEADER" ]; then
  curl -sS $CURL_OPTS -o /dev/null -D "$HEADERS_A" -H "Host: $AIRFLOW_HOST" "https://127.0.0.1/auth/login/keycloak" 2>/dev/null || true
else
  curl -sS $CURL_OPTS -o /dev/null -D "$HEADERS_A" "https://$AIRFLOW_HOST/auth/login/keycloak" 2>/dev/null || true
fi
LOC_A=$(grep -i '^Location:' "$HEADERS_A" | head -1 | sed 's/^[Ll]ocation:[[:space:]]*//' | tr -d '\r\n')
CODE_A=$(grep -E '^HTTP' "$HEADERS_A" | tail -1 | awk '{print $2}')
echo "    HTTP $CODE_A"
if [ -n "$LOC_A" ]; then
  echo "    Location: ${LOC_A:0:120}..."
  if echo "$LOC_A" | grep -q 'redirect_uri=http%3A%2F%2F'; then
    echo -e "    ${RED}FAIL: redirect_uri is HTTP (Keycloak will 400)${NC}"
    FAILED=1
  else
    echo -e "    ${GREEN}OK: redirect_uri is HTTPS${NC}"
  fi
  if echo "$LOC_A" | grep -q 'error='; then
    echo -e "    ${RED}FAIL: Location contains error= (Keycloak error in redirect)${NC}"
    FAILED=1
  fi
else
  echo -e "    ${YELLOW}WARN: no Location header${NC}"
fi
rm -f "$HEADERS_A"

# --- STOCK WATCHLIST ---
echo ""
echo "========== 3. STOCK SERVICE WATCHLIST =========="
WATCHLIST_PATH="/api/v1/stock-service/watchlist"
if [ -n "$USE_HOST_HEADER" ]; then
  WATCH_CODE=$(curl -sS $CURL_OPTS -o /tmp/e2e_watchlist.json -w "%{http_code}" -H "Host: ${BASE_URL#*://}" "https://127.0.0.1$WATCHLIST_PATH" 2>/dev/null || echo "000")
  WATCH_HOST="${BASE_URL#*://}"
  WATCH_HOST="${WATCH_HOST%%/*}"
else
  WATCH_CODE=$(curl -sS $CURL_OPTS -o /tmp/e2e_watchlist.json -w "%{http_code}" "$BASE_URL$WATCHLIST_PATH" 2>/dev/null || echo "000")
fi
echo "3.1 GET watchlist: HTTP $WATCH_CODE"
if [ "$WATCH_CODE" = "200" ]; then
  if command -v jq >/dev/null 2>&1; then
    FIRST_ERR=$(jq -r '.symbols[0].error // "none"' /tmp/e2e_watchlist.json 2>/dev/null || echo "?")
    FIRST_PRICE=$(jq -r '.symbols[0].price // "null"' /tmp/e2e_watchlist.json 2>/dev/null)
  else
    FIRST_ERR=$(grep -o '"error":"[^"]*"' /tmp/e2e_watchlist.json 2>/dev/null | head -1 | sed 's/"error":"//;s/"$//') || echo "?"
    FIRST_PRICE="(check JSON)"
  fi
  echo "    First symbol: price=$FIRST_PRICE, error=$FIRST_ERR"
  if [ "$FIRST_ERR" != "none" ] && [ -n "$FIRST_ERR" ]; then
    echo -e "    ${YELLOW}WARN: API returns error (e.g. IBKR no bridge – run Client Portal Gateway for prices)${NC}"
  else
    echo -e "    ${GREEN}OK: Watchlist returned data${NC}"
  fi
else
  echo -e "    ${RED}FAIL: watchlist HTTP $WATCH_CODE${NC}"
  FAILED=1
fi

echo ""
echo "=============================================="
if [ $FAILED -eq 1 ]; then
  echo -e "${RED}E2E Full Trace: FAILED (see above)${NC}"
  exit 1
fi
echo -e "${GREEN}E2E Full Trace: PASSED${NC}"
exit 0
