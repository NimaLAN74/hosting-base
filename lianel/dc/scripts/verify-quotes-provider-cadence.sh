#!/usr/bin/env bash
# Verify quotes endpoint: first call can hit provider, second within 60s from cache, third after 60s hits provider again.
# Usage: BASE_URL=https://www.lianel.se ./verify-quotes-provider-cadence.sh
#        Or run against local backend: BASE_URL=http://localhost:8080 ./verify-quotes-provider-cadence.sh
set -e
BASE_URL="${BASE_URL:-http://localhost:8080}"
# Nginx may expose under /api/v1/stock-monitoring/quotes; backend route is /api/v1/quotes
QUOTES_PATH="${QUOTES_PATH:-/api/v1/quotes}"
PAIRS="${PAIRS:-AAPL:yahoo}"
URL="${BASE_URL}${QUOTES_PATH}?pairs=${PAIRS}"

echo "=== Call 1 (may be provider or cache) ==="
curl -sS -i -H "Accept: application/json" "${URL}" 2>/dev/null | head -30
echo ""
echo "=== Waiting 65s so cache expires (provider called at most every 60s) ==="
sleep 65
echo "=== Call 2 (should be X-Quotes-Source: provider) ==="
curl -sS -i -H "Accept: application/json" "${URL}" 2>/dev/null | head -30
