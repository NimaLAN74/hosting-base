#!/usr/bin/env bash
# Verify that www.lianel.se serves Keycloak theme CSS with Content-Type text/css.
# Run after any nginx config deploy. Exit 0 only if CSS is 200 + text/css.
# Usage: run from repo root or with BASE_URL default https://www.lianel.se
set -e

BASE_URL="${BASE_URL:-https://www.lianel.se}"
CSS_URL="${BASE_URL}/resources/lbvvu/common/keycloak/vendor/patternfly-v5/patternfly.min.css"

echo "=== Verify Keycloak theme CSS on www ==="
echo "  URL: $CSS_URL"

HTTP=$(curl -s -k -o /tmp/verify_kc_css.bin -w "%{http_code}" "$CSS_URL")
CT=$(curl -s -k -I "$CSS_URL" 2>/dev/null | grep -i "^Content-Type:" | tr -d '\r' | cut -d' ' -f2-)

if [ "$HTTP" != "200" ]; then
  echo "  FAIL: HTTP $HTTP (expected 200)"
  exit 1
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
