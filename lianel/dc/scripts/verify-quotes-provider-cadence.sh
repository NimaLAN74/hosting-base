#!/usr/bin/env bash
# Verify quotes: use symbol+provider pairs from the data source (watchlist_items) and call quotes.
# On server: use /internal/quotes so backend loads pairs from DB (no need to pass pairs=).
# Usage (on server via SSH):
#   ssh root@SERVER "docker exec lianel-stock-service curl -sS -i -H 'Accept: application/json' 'http://localhost:3003/internal/quotes'"
# Or run this script on the server with USE_DATA_SOURCE=1 (pairs come from DB):
#   USE_DATA_SOURCE=1 BASE_URL=http://localhost:3003 ./verify-quotes-provider-cadence.sh
# With explicit pairs (e.g. local): PAIRS=MSFT:finnhub,AAPL:yahoo ./verify-quotes-provider-cadence.sh
set -e
BASE_URL="${BASE_URL:-http://localhost:8080}"
USE_DATA_SOURCE="${USE_DATA_SOURCE:-0}"

if [ "$USE_DATA_SOURCE" = "1" ]; then
  # Pairs loaded from watchlist_items in the backend; no query string.
  QUOTES_PATH="${QUOTES_PATH:-/internal/quotes}"
  URL="${BASE_URL}${QUOTES_PATH}"
  echo "=== Using pairs from data source (watchlist_items) ==="
else
  QUOTES_PATH="${QUOTES_PATH:-/api/v1/quotes}"
  PAIRS="${PAIRS:-AAPL:yahoo}"
  URL="${BASE_URL}${QUOTES_PATH}?pairs=${PAIRS}"
fi

echo "=== Call 1 (may be provider or cache) ==="
curl -sS -i -H "Accept: application/json" "${URL}" 2>/dev/null | head -40
echo ""
echo "=== Waiting 65s so cache expires (provider called at most every 60s) ==="
sleep 65
echo "=== Call 2 (should be X-Quotes-Source: provider) ==="
curl -sS -i -H "Accept: application/json" "${URL}" 2>/dev/null | head -40
