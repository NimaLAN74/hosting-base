#!/bin/bash
# E2E: Stock monitoring routes and API (no browser). Run from CI or repo root.
# Verifies: /stock-app -> 302 to /stock/, /stock/ -> 200 (main app HTML), stock API health.
set -e

BASE_URL="${BASE_URL:-https://www.lianel.se}"
FAILED=0

echo "=== E2E Stock Service (routes + API) ==="
echo "BASE_URL=$BASE_URL"
echo ""

# 1) /stock-app must redirect to /stock/ (no -L so we see the redirect response)
echo "1. GET /stock-app (expect 302 to /stock/)..."
REDIR_OUT=$(curl -sS -o /dev/null -w "%{http_code}" -D - "$BASE_URL/stock-app" 2>/dev/null)
CODE=$(echo "$REDIR_OUT" | tail -1)
LOCATION=$(echo "$REDIR_OUT" | grep -i '^Location:' | head -1 | tr -d '\r' | cut -d' ' -f2-)
if [ "$CODE" = "302" ] && echo "$LOCATION" | grep -q '/stock'; then
  echo "   ✅ /stock-app -> 302 Location: $LOCATION"
elif [ "$CODE" = "200" ]; then
  echo "   ⚠️  /stock-app returned 200 (expected 302). Nginx redirect may not be deployed."
  FAILED=1
else
  echo "   ❌ /stock-app HTTP $CODE (expected 302), Location: $LOCATION"
  FAILED=1
fi
echo ""

# 2) /stock/ must return 200 and main app HTML
echo "2. GET /stock/ (expect 200, main app HTML)..."
BODY=$(curl -sS "$BASE_URL/stock/" 2>/dev/null)
CODE=$(curl -sS -o /dev/null -w "%{http_code}" "$BASE_URL/stock/" 2>/dev/null)
if [ "$CODE" = "200" ] && echo "$BODY" | grep -q 'Lianel World\|root\|static/js'; then
  echo "   ✅ /stock/ -> 200, main app HTML"
else
  echo "   ❌ /stock/ HTTP $CODE or wrong body (expected 200 + main app)"
  FAILED=1
fi
echo ""

# 3) Stock API health (public)
echo "3. GET /api/v1/stock-service/health (expect 200)..."
HEALTH_CODE=$(curl -sS -o /dev/null -w "%{http_code}" "$BASE_URL/api/v1/stock-service/health" 2>/dev/null)
if [ "$HEALTH_CODE" = "200" ]; then
  echo "   ✅ Stock API health: 200"
else
  echo "   ❌ Stock API health: HTTP $HEALTH_CODE"
  FAILED=1
fi
echo ""

if [ $FAILED -eq 1 ]; then
  echo "=== E2E Stock Service: FAILED ==="
  exit 1
fi
echo "=== E2E Stock Service: PASSED ==="
exit 0
