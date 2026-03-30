#!/bin/bash
# E2E: Call stock-service price-history and quotes APIs, then report why charts may be incomplete.
# Run ON the remote server. Uses docker exec to hit the backend from inside the stack.
#
# Usage (on server):
#   docker exec lianel-stock-service bash -c 'curl -s -H "x-auth-request-user: test" "http://localhost:3003/api/v1/price-history?symbol=SHL.L&days=7"' | jq .
# Or run this script on the host with CONTAINER=lianel-stock-service and call backend via docker exec.
#
# Required on host: curl, jq (optional but recommended). CONTAINER can be overridden.

set -e

CONTAINER="${CONTAINER:-lianel-stock-service}"
BASE="http://localhost:3003/api/v1"
AUTH="x-auth-request-user: test"
SYMBOLS="${E2E_SYMBOLS:-SHL.L ASML.AS SAP.DE AAPL}"

echo "=== E2E Stock Chart API ==="
echo "Container: $CONTAINER"
echo "Symbols: $SYMBOLS"
echo ""

# 1) Price-history for each symbol
echo "--- Price history (daily + intraday_today) ---"
daily_total=0
intra_total=0
for sym in $SYMBOLS; do
  json=$(docker exec "$CONTAINER" curl -s -H "$AUTH" "${BASE}/price-history?symbol=${sym}&days=7" 2>/dev/null || echo "{}")
  if [ -z "$json" ] || [ "$json" = "{}" ]; then
    echo "  $sym: no response"
    continue
  fi
  daily_n=$(echo "$json" | jq -r '.daily | length' 2>/dev/null || echo "0")
  intra_n=$(echo "$json" | jq -r '.intraday_today | length' 2>/dev/null || echo "0")
  daily_total=$((daily_total + daily_n))
  intra_total=$((intra_total + intra_n))
  echo "  $sym: daily=$daily_n intraday_today=$intra_n"
done
echo ""

# 2) Quotes and per-quote source
echo "--- Quotes (provider source per symbol) ---"
symbols_comma=$(echo "$SYMBOLS" | tr ' ' ',')
quotes_json=$(docker exec "$CONTAINER" curl -s -H "$AUTH" "${BASE}/quotes?symbols=${symbols_comma}" 2>/dev/null || echo "{}")
if [ -n "$quotes_json" ] && [ "$quotes_json" != "{}" ]; then
  echo "$quotes_json" | jq -r '.quotes[]? | "  \(.symbol): price=\(.price) source=\(.source // "n/a")"' 2>/dev/null || echo "  (parse error)"
else
  echo "  No quotes response"
fi
echo ""

# 3) Root cause summary for chart completeness
echo "--- Chart completeness diagnosis ---"
if [ "$daily_total" -eq 0 ] && [ "$intra_total" -eq 0 ]; then
  echo "  Charts show little/no history because:"
  echo "    - daily=0: No rows in price_history_daily (roll-daily job not run or no intraday data yet)."
  echo "    - intraday_today=0: No rows in price_history_intraday for today, and no Redis/cache points."
  echo "  Fix: Confirm stock-service has history data (IBKR/watchlist path); set REDIS_URL if using Redis; legacy Airflow roll/ingest DAGs were removed."
elif [ "$daily_total" -eq 0 ]; then
  echo "  Last-7-days chart incomplete: daily=0 (roll-daily job not run or no data). Current-day chart can show intraday points."
  echo "  Fix: Populate price_history from the current stock-service pipeline (no Airflow roll DAG)."
elif [ "$intra_total" -eq 0 ]; then
  echo "  Current-day chart empty: intraday_today=0. Last-7-days chart may show daily bars only."
  echo "  Fix: Ensure watchlist quotes and persist/Redis writes succeed (dashboard or stock_service_heartbeat for connectivity)."
else
  echo "  API returns both daily and intraday data; chart should show history. If UI still incomplete, check frontend parsing (trade_date/observed_at, close_price/price)."
fi
echo "Done."
