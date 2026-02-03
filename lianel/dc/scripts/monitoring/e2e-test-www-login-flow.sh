#!/usr/bin/env bash
# E2E test: login flow via www.lianel.se (same as the frontend).
# Verifies: nginx routes /auth/ to Keycloak, PKCE works, login page is Keycloak HTML (not React), form/theme use www.
set -e

BASE_URL="${BASE_URL:-https://www.lianel.se}"
AUTH_BASE="${BASE_URL}/auth"
REALM="${REALM:-lianel}"
CLIENT_ID="${CLIENT_ID:-frontend-client}"
REDIRECT_URI="${REDIRECT_URI:-https://www.lianel.se/}"

# Generate PKCE code_verifier and code_challenge (S256)
code_verifier=$(openssl rand -base64 32 | tr -d '=\n' | tr '+/' '-_')
code_challenge=$(echo -n "$code_verifier" | openssl dgst -binary -sha256 | openssl base64 | tr -d '=\n' | tr '+/' '-_')

AUTH_URL="${AUTH_BASE}/realms/${REALM}/protocol/openid-connect/auth"
PARAMS="client_id=${CLIENT_ID}&redirect_uri=${REDIRECT_URI}&response_type=code&scope=openid%20profile%20email&prompt=login&code_challenge_method=S256&code_challenge=${code_challenge}&state=e2e-test"

echo "=== E2E Test: Login flow via www.lianel.se ==="
echo "  Auth URL: ${AUTH_URL}"
echo "  PKCE: S256, challenge length ${#code_challenge}"
echo ""

# Step 1: Request auth URL (no redirect) – expect 302 (redirect to login) or 200 (login page directly)
echo "1. GET auth URL (no redirect)..."
HTTP=$(curl -s -k -o /tmp/e2e_login.html -w "%{http_code}" -D /tmp/e2e_headers.txt "${AUTH_URL}?${PARAMS}")
LOCATION=$(grep -i '^Location:' /tmp/e2e_headers.txt 2>/dev/null | tr -d '\r' | cut -d' ' -f2-)

if [ "$HTTP" = "302" ]; then
  echo "   OK: 302 redirect"
  if echo "$LOCATION" | grep -q "error=invalid_request\|error=access_denied"; then
    echo "   FAIL: Keycloak error redirect: $LOCATION"
    exit 1
  fi
  # Follow redirect to get login page body
  curl -s -k -L -o /tmp/e2e_login_page.html "${AUTH_URL}?${PARAMS}"
elif [ "$HTTP" = "200" ]; then
  echo "   OK: 200 (login page returned directly)"
  cp /tmp/e2e_login.html /tmp/e2e_login_page.html
else
  echo "   FAIL: expected 200 or 302, got $HTTP"
  head -20 /tmp/e2e_login.html
  exit 1
fi

echo ""
echo "2. Checking login page content..."
SIZE=$(wc -c < /tmp/e2e_login_page.html)

# Step 4: Must be Keycloak HTML (large, has login form), not React (tiny index.html)
if [ "$SIZE" -lt 3000 ]; then
  echo "   FAIL: response too small ($SIZE bytes). Expected Keycloak login HTML (>3KB), got likely React index.html."
  head -5 /tmp/e2e_login_page.html
  exit 1
fi
echo "   OK: response size $SIZE bytes (Keycloak HTML)"

# Step 5: Must contain Keycloak login form markers
if ! grep -q 'login-actions/authenticate\|id="kc-form-login"\|name="username"\|Sign in' /tmp/e2e_login_page.html 2>/dev/null; then
  echo "   FAIL: page does not look like Keycloak login (no form/username/Sign in)"
  grep -o 'action="[^"]*"\|<title>[^<]*</title>' /tmp/e2e_login_page.html | head -5
  exit 1
fi
echo "   OK: Keycloak login form present"

# Step 6: Form action must be same-origin (www.lianel.se or /auth/...) so POST does not 400
FORM_ACTION=$(grep -oE 'action="[^"]*"' /tmp/e2e_login_page.html | head -1)
if echo "$FORM_ACTION" | grep -q 'auth.lianel.se'; then
  echo "   FAIL: form action uses auth.lianel.se (login POST will return 400). Set KC_HOSTNAME=https://www.lianel.se/auth and restart Keycloak."
  echo "   Form: $FORM_ACTION"
  exit 1
elif echo "$FORM_ACTION" | grep -q 'www.lianel.se\|/auth/'; then
  echo "   OK: form action same-origin (www or /auth)"
else
  echo "   INFO: form action: $FORM_ACTION"
fi

# Step 7: Theme/resources should point to www or relative (no auth.lianel.se = no MIME issue)
RESOURCE_LINKS=$(grep -oE 'href="[^"]*resources[^"]*"|src="[^"]*resources[^"]*"' /tmp/e2e_login_page.html | head -3)
if [ -n "$RESOURCE_LINKS" ]; then
  if echo "$RESOURCE_LINKS" | grep -q 'auth.lianel.se'; then
    echo "   WARN: some theme URLs use auth.lianel.se (possible MIME/CORS)"
  else
    echo "   OK: theme URLs relative or www"
  fi
fi

# Step 8: Fetch one theme CSS and check Content-Type (must be text/css, not text/html)
THEME_CSS_PATH=$(grep -oE 'href="(/[^"]*\.css)"' /tmp/e2e_login_page.html | head -1 | sed 's/href="\(.*\)"/\1/')
if [ -n "$THEME_CSS_PATH" ]; then
  echo ""
  echo "3. GET theme CSS (must be text/css)..."
  CT=$(curl -s -k -o /tmp/e2e_theme.css -w "%{content_type}" "${BASE_URL}${THEME_CSS_PATH}")
  CSS_HTTP=$(curl -s -k -o /dev/null -w "%{http_code}" "${BASE_URL}${THEME_CSS_PATH}")
  if [ "$CSS_HTTP" != "200" ]; then
    echo "   FAIL: CSS request returned HTTP $CSS_HTTP (expected 200)"
    exit 1
  fi
  if echo "$CT" | grep -q "text/css"; then
    echo "   OK: Content-Type is text/css ($CT)"
  else
    echo "   FAIL: Content-Type is '$CT' (expected text/css). Theme CSS would be blocked by browser."
    exit 1
  fi
fi

echo ""
echo "=== E2E result: PASS – login flow via www.lianel.se is correct ==="
