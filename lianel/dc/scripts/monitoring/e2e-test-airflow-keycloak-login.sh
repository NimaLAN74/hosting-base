#!/bin/bash
# E2E test: Airflow + Keycloak login flow (redirect_uri must be https).
# Run from repo root or lianel/dc: bash scripts/monitoring/e2e-test-airflow-keycloak-login.sh
# Can run from local machine (curls public URLs).

set -e

AIRFLOW_URL="${AIRFLOW_URL:-https://airflow.lianel.se}"
AUTH_URL="${AUTH_URL:-https://auth.lianel.se}"
COOKIE_JAR=$(mktemp)
trap "rm -f $COOKIE_JAR" EXIT

echo "=========================================="
echo "E2E: Airflow + Keycloak Login Flow"
echo "=========================================="
echo ""
echo "Airflow: $AIRFLOW_URL"
echo "Keycloak: $AUTH_URL"
echo ""

# 1. Hit Airflow login page (get session cookie)
echo "1. GET $AIRFLOW_URL/auth/login/"
HTTP_LOGIN=$(curl -sS -o /tmp/airflow_login.html -w "%{http_code}" -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
  -L --max-redirs 5 \
  "$AIRFLOW_URL/auth/login/" 2>&1)
echo "   HTTP $HTTP_LOGIN"

if [ "$HTTP_LOGIN" != "200" ] && [ "$HTTP_LOGIN" != "302" ]; then
  echo "   ❌ Login page returned $HTTP_LOGIN (expected 200 or 302)"
  echo "   First 3 lines of response:"
  head -3 /tmp/airflow_login.html
  exit 1
fi
echo "   ✅ Login page reachable"

# 2. Request Keycloak OAuth initiation (FAB uses /auth/login/<provider>)
#    This redirects to Keycloak with redirect_uri in query string
echo ""
echo "2. GET $AIRFLOW_URL/auth/login/keycloak (expect redirect to Keycloak)"
REDIRECT_HEADERS=$(mktemp)
trap "rm -f $COOKIE_JAR $REDIRECT_HEADERS" EXIT

# Do not follow redirects so we can read the Location header
HTTP_OAUTH=$(curl -sS -o /dev/null -w "%{http_code}" -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
  -D "$REDIRECT_HEADERS" \
  "$AIRFLOW_URL/auth/login/keycloak" 2>&1)

LOCATION=$(grep -i "^Location:" "$REDIRECT_HEADERS" | head -1 | tr -d '\r\n')
echo "   HTTP $HTTP_OAUTH"
echo "   $LOCATION"

if [ -z "$LOCATION" ]; then
  echo "   ❌ No Location header (expected redirect to Keycloak)"
  echo "   Headers:"
  cat "$REDIRECT_HEADERS"
  exit 1
fi

REDIRECT_URL=$(echo "$LOCATION" | sed 's/^[Ll]ocation:[[:space:]]*//' | tr -d '\r\n')
echo "   Redirect URL: $REDIRECT_URL"

# 3. Check redirect_uri in the URL (must be https)
if echo "$REDIRECT_URL" | grep -q 'redirect_uri=http%3A%2F%2F'; then
  echo ""
  echo "   ❌ FAIL: redirect_uri is HTTP (Keycloak will return 400)"
  echo "   Decoded: redirect_uri=http://..."
  echo ""
  echo "   This is the Airflow Keycloak integration bug: FAB is building redirect_uri"
  echo "   from request scheme (http behind proxy). Fix: ensure webserver_config.py"
  echo "   before_request forces wsgi.url_scheme=https and is actually registered."
  exit 1
fi

if echo "$REDIRECT_URL" | grep -q 'redirect_uri=https%3A%2F%2F'; then
  echo "   ✅ redirect_uri is HTTPS (correct)"
else
  echo "   ⚠️  Could not find redirect_uri in URL; inspect manually"
fi

# 4. Follow to Keycloak and check we don't get 400
echo ""
echo "3. Following redirect to Keycloak (check for 400 Bad Request)"
HTTP_KEYCLOAK=$(curl -sS -o /tmp/keycloak_page.html -w "%{http_code}" -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
  -L --max-redirs 3 \
  "$REDIRECT_URL" 2>&1)
echo "   Keycloak response: HTTP $HTTP_KEYCLOAK"

if [ "$HTTP_KEYCLOAK" = "400" ]; then
  echo "   ❌ Keycloak returned 400 (likely invalid redirect_uri)"
  echo "   Response body (first 20 lines):"
  head -20 /tmp/keycloak_page.html
  exit 1
fi

if [ "$HTTP_KEYCLOAK" = "200" ]; then
  echo "   ✅ Keycloak login page reached (200)"
  if grep -qi "sign in\|login\|keycloak" /tmp/keycloak_page.html 2>/dev/null; then
    echo "   ✅ Page looks like Keycloak login"
  fi
else
  echo "   ⚠️  Keycloak returned $HTTP_KEYCLOAK (expected 200 for login page)"
fi

# 5. Callback URL reachable (without code: expect redirect to login or 400, not 500)
echo ""
echo "4. GET callback URL without code (expect 302 or 400, not 500)"
HTTP_CALLBACK=$(curl -sS -o /tmp/callback.html -w "%{http_code}" -c "$COOKIE_JAR" -b "$COOKIE_JAR" \
  "$AIRFLOW_URL/auth/oauth-authorized/keycloak" 2>&1)
echo "   Callback response: HTTP $HTTP_CALLBACK"

if [ "$HTTP_CALLBACK" = "500" ]; then
  echo "   ❌ Callback returned 500 (check apiserver logs; AIRFLOW_OAUTH_CLIENT_SECRET?)"
  echo "   Response (first 15 lines):"
  head -15 /tmp/callback.html
  exit 1
fi
if [ "$HTTP_CALLBACK" = "302" ] || [ "$HTTP_CALLBACK" = "400" ] || [ "$HTTP_CALLBACK" = "200" ]; then
  echo "   ✅ Callback endpoint reachable (HTTP $HTTP_CALLBACK)"
else
  echo "   ⚠️  Callback returned $HTTP_CALLBACK"
fi

echo ""
echo "=========================================="
echo "E2E Result: Airflow + Keycloak flow OK"
echo "=========================================="
