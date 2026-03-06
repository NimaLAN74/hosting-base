#!/bin/bash
# Run ON the remote server (or in a container that can reach the stock-service backend).
# Calls the price-history API for each symbol (days=7) and prints daily + intraday_today
# so you can compare with what the chart shows.
#
# One API: GET /api/v1/price-history?symbol=SYMBOL&days=7
# Returns: daily (last 7 days, close_price per day) + intraday_today (today's points).
# Chart "Last 7 days" = daily + intraday_today + session + Now.
# Chart "Current day"  = intraday_today + session + Now.
#
# Usage: SYMBOLS="SHL.L ASML.AS SAP.DE" bash price-history-api-compare.sh
#   or   bash price-history-api-compare.sh SHL.L ASML.AS

set -e

SYMBOLS="${SYMBOLS:-SHL.L ASML.AS SAP.DE}"
[ $# -gt 0 ] && SYMBOLS="$*"

BASE_URL="${PRICE_HISTORY_URL:-http://localhost:3003/api/v1/price-history}"
AUTH_HEADER="${PRICE_HISTORY_AUTH_HEADER:-x-auth-request-user: test}"

echo "=== Price history API comparison (daily + 7-day from same endpoint) ==="
echo "Endpoint: $BASE_URL?symbol=SYMBOL&days=7"
echo "Symbols: $SYMBOLS"
echo ""

for sym in $SYMBOLS; do
  echo "--- Symbol: $sym ---"
  json=$(curl -s -H "$AUTH_HEADER" "${BASE_URL}?symbol=${sym}&days=7" 2>/dev/null || echo "{}")
  if [ -z "$json" ] || [ "$json" = "{}" ]; then
    echo "  No response or empty"
    echo ""
    continue
  fi

  if command -v jq >/dev/null 2>&1; then
    echo "  daily (for Last 7 days chart):"
    echo "$json" | jq -r '.daily[]? | "    \(.trade_date)  close=\(.close_price)"' 2>/dev/null || echo "    (jq parse failed)"
    daily_count=$(echo "$json" | jq -r '.daily | length' 2>/dev/null || echo "0")
    echo "  daily count: $daily_count"
    echo "  intraday_today (for Current day chart):"
    echo "$json" | jq -r '.intraday_today[]? | "    \(.observed_at)  price=\(.price)"' 2>/dev/null || echo "    (jq parse failed)"
    intra_count=$(echo "$json" | jq -r '.intraday_today | length' 2>/dev/null || echo "0")
    echo "  intraday_today count: $intra_count"
  else
    echo "  (install jq for parsed output; raw response below)"
    echo "$json"
  fi
  echo "  Chart comparison: Last 7 days = daily (${daily_count:-?}) + intraday_today (${intra_count:-?}) + session + Now."
  echo "                    Current day = intraday_today (${intra_count:-?}) + session + Now."
  echo ""
done

echo "Done."
