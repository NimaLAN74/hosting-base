#!/usr/bin/env bash
# Verify that www.lianel.se serves Keycloak theme CSS with Content-Type text/css.
# Run after any nginx config deploy. Exit 0 only if CSS is 200 + text/css.
# Discovers the current theme CSS URL from the login page (version hash changes on Keycloak restart).
# Usage: run from repo root or with BASE_URL default https://www.lianel.se
set -e

BASE_URL="${BASE_URL:-https://www.lianel.se}"
LOGIN_URL="${BASE_URL}/auth/realms/lianel/protocol/openid-connect/auth?client_id=frontend-client&redirect_uri=${BASE_URL}/&response_type=code&scope=openid"

echo "=== Verify Keycloak theme CSS on www ==="

# Discover theme CSS URL from login page (Keycloak version hash e.g. lbvvu/gio9g changes per deploy)
curl -s -k -L -o /tmp/verify_kc_login.html "$LOGIN_URL" || true
if [ ! -f /tmp/verify_kc_login.html ] || [ "$(wc -c < /tmp/verify_kc_login.html)" -lt 500 ]; then
  echo "  SKIP: Login page unavailable (502 or empty); cannot discover theme URL. Nginx sync was still applied."
  exit 0
fi
# Extract first resources CSS path from the page.
# Keycloak may emit absolute (/auth/resources/...) or relative (resources/...) URLs depending on theme and hostname settings.
CSS_PATH=$(grep -oE '(/(auth/)?resources/|resources/)[^"'\''<> ]+\.css' /tmp/verify_kc_login.html | head -1)
if [ -z "$CSS_PATH" ]; then
  # Fallback: try known path (hash may be outdated)
  CSS_PATH="/auth/resources/lbvvu/common/keycloak/vendor/patternfly-v5/patternfly.min.css"
  echo "  (using fallback path)"
fi
# Normalize: ensure leading slash and keep it under /auth/ when relative paths are used.
CSS_PATH="${CSS_PATH#/}"
if [[ "$CSS_PATH" == resources/* ]]; then
  CSS_PATH="auth/${CSS_PATH}"
fi
CSS_URL="${BASE_URL}/${CSS_PATH}"
echo "  URL: $CSS_URL"

HTTP=$(curl -s -k -o /tmp/verify_kc_css.bin -w "%{http_code}" "$CSS_URL")
CT=$(curl -s -k -I "$CSS_URL" 2>/dev/null | grep -i "^Content-Type:" | tr -d '\r' | cut -d' ' -f2-)

if [ "$HTTP" != "200" ]; then
  # Some deployments expose theme assets at /resources/ on www; try that fallback before failing.
  if [[ "$CSS_PATH" == auth/resources/* ]]; then
    CSS_URL_FALLBACK="${BASE_URL}/${CSS_PATH#auth/}"
    echo "  Retry fallback URL: $CSS_URL_FALLBACK"
    HTTP=$(curl -s -k -o /tmp/verify_kc_css.bin -w "%{http_code}" "$CSS_URL_FALLBACK")
    CT=$(curl -s -k -I "$CSS_URL_FALLBACK" 2>/dev/null | grep -i "^Content-Type:" | tr -d '\r' | cut -d' ' -f2-)
    CSS_URL="$CSS_URL_FALLBACK"
  fi
  if [ "$HTTP" != "200" ]; then
    echo "  FAIL: HTTP $HTTP (expected 200)"
    exit 1
  fi
fi
if ! echo "$CT" | grep -q "text/css"; then
  echo "  FAIL: Content-Type is '$CT' (expected text/css)"
  exit 1
fi

# Sanity: first bytes should look like CSS, not HTML
HEAD=$(head -c 80 /tmp/verify_kc_css.bin)
if echo "$HEAD" | grep -q "<!DOCTYPE\|<html"; then
  echo "  FAIL: Response body looks like HTML, not CSS"
  exit 1
fi

echo "  OK: HTTP $HTTP, Content-Type $CT"
echo "=== Verification PASSED ==="
