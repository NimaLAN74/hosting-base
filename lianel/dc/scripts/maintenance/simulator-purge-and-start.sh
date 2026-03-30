#!/usr/bin/env bash
# Purge all simulator runs in Redis (requires SIMULATOR_PURGE_SECRET on stock-service),
# then POST a new run. For operators / smoke tests.
#
# Usage:
#   export STOCK_SERVICE_URL=https://www.lianel.se   # or http://127.0.0.1:3003
#   export SIMULATOR_PURGE_SECRET='your-secret'
#   ./simulator-purge-and-start.sh
#
# Optional: SIM_START_JSON overrides the POST body (single-line JSON).

set -euo pipefail
BASE="${STOCK_SERVICE_URL:-http://127.0.0.1:3003}"
BASE="${BASE%/}"
if [[ -z "${SIMULATOR_PURGE_SECRET:-}" ]]; then
  echo "error: set SIMULATOR_PURGE_SECRET in the environment" >&2
  exit 1
fi

PURGE_BODY=$(SIMULATOR_PURGE_SECRET="$SIMULATOR_PURGE_SECRET" python3 -c 'import json, os; print(json.dumps({"secret": os.environ["SIMULATOR_PURGE_SECRET"]}))')

echo "POST ${BASE}/api/v1/stock-service/sim/purge"
PURGE_OUT=$(curl -sS -X POST "${BASE}/api/v1/stock-service/sim/purge" \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json' \
  -d "$PURGE_BODY")
echo "$PURGE_OUT" | python3 -m json.tool 2>/dev/null || echo "$PURGE_OUT"
python3 -c "import json,sys; d=json.loads(sys.argv[1]); sys.exit(0 if d.get('ok') else 1)" "$PURGE_OUT" || {
  echo "error: purge failed" >&2
  exit 1
}

START_JSON="${SIM_START_JSON:-{\"days\":7,\"top\":16,\"quantile\":0.2,\"short_enabled\":true,\"initial_capital_usd\":100,\"replay_delay_ms\":50,\"reinvest_profit\":true,\"live_market_data\":false,\"max_cycles\":2000,\"readiness_min_days\":126}}"
echo "POST ${BASE}/api/v1/stock-service/sim/runs"
START_OUT=$(curl -sS -X POST "${BASE}/api/v1/stock-service/sim/runs" \
  -H 'Content-Type: application/json' \
  -H 'Accept: application/json' \
  -d "$START_JSON")
echo "$START_OUT" | python3 -m json.tool 2>/dev/null || echo "$START_OUT"
RUN_ID=$(echo "$START_OUT" | python3 -c "import sys,json; print(json.load(sys.stdin).get('run_id',''))" 2>/dev/null || true)
if [[ -z "$RUN_ID" ]]; then
  echo "error: start run failed" >&2
  exit 1
fi

echo "Polling status for $RUN_ID ..."
for _ in $(seq 1 15); do
  sleep 2
  ST=$(curl -sS "${BASE}/api/v1/stock-service/sim/runs/${RUN_ID}/status")
  python3 -c "import json,sys; d=json.loads(sys.argv[1]); print('status=', d.get('status'), 'cycles=', d.get('cycles_completed'))" "$ST" || echo "$ST"
  S=$(echo "$ST" | python3 -c "import sys,json; print(json.load(sys.stdin).get('status',''))" 2>/dev/null || echo "")
  if [[ "$S" == "completed" || "$S" == "failed" ]]; then
    break
  fi
done
echo "done."
