#!/usr/bin/env bash
# Verify Comp-AI history API and UX.
# Run on remote host or locally; requires a valid Bearer token for the Comp-AI API.
# Get a token via Keycloak (e.g. log in to www.lianel.se and copy from browser devtools / Network).
set -e

BASE_URL="${COMP_AI_BASE_URL:-https://www.lianel.se}"
TOKEN="${COMP_AI_TOKEN:-}"

if [ -z "$TOKEN" ]; then
  echo "Usage: COMP_AI_TOKEN=<bearer_token> $0"
  echo "  Or: export COMP_AI_TOKEN then run $0"
  echo "Get a token: log in to ${BASE_URL}, open Comp AI, then DevTools → Network → find request to /api/v1/comp-ai/process → copy Authorization header value (Bearer ...)."
  exit 1
fi

echo "Checking Comp-AI health..."
curl -sf "${BASE_URL}/api/v1/comp-ai/health" || true
echo ""

echo "Fetching request history (limit=5)..."
HTTP=$(curl -s -o /tmp/comp_ai_history.json -w "%{http_code}" \
  -H "Authorization: Bearer ${TOKEN}" \
  "${BASE_URL}/api/v1/comp-ai/history?limit=5&offset=0")
if [ "$HTTP" = "200" ]; then
  echo "History API returned 200 OK"
  cat /tmp/comp_ai_history.json | head -c 500
  echo "..."
else
  echo "History API returned HTTP $HTTP"
  cat /tmp/comp_ai_history.json
fi
