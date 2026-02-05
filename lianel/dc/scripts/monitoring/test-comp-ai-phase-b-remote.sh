#!/usr/bin/env bash
# Run on the remote host (e.g. via SSH) to test Phase B: evidence upload and analyse.
# Uses .env for Keycloak; calls comp-ai-service at http://lianel-comp-ai-service:3002 (from host use
# the same URL if on Docker network, or http://127.0.0.1:3002 if port is published).
# Usage (on remote): cd /root/hosting-base/lianel/dc && bash scripts/monitoring/test-comp-ai-phase-b-remote.sh
# Or via SSH: ssh root@72.60.80.84 'cd /root/hosting-base/lianel/dc && bash scripts/monitoring/test-comp-ai-phase-b-remote.sh'
# Or run from pipeline: Actions → Run Comp-AI Phase B test on remote

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DC_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$DC_DIR"

for env in .env ../.env /root/lianel/dc/.env /root/hosting-base/lianel/dc/.env; do
  [ -f "$env" ] && source "$env" && break
done

KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
REALM="${KEYCLOAK_REALM:-lianel}"
CLIENT_ID="${COMP_AI_KEYCLOAK_CLIENT_ID:-comp-ai-service}"
CLIENT_SECRET="${COMP_AI_KEYCLOAK_CLIENT_SECRET:-}"
# When run on host: comp-ai port is not published; use DOCKER_NETWORK and curl via container.
# Set COMP_AI_BASE_URL=http://lianel-comp-ai-service:3002 and CURL_VIA_DOCKER=1 to run API calls in a container on same network.
BASE_URL="${COMP_AI_BASE_URL:-http://lianel-comp-ai-service:3002}"
CURL_VIA_DOCKER="${CURL_VIA_DOCKER:-1}"
DOCKER_NETWORK="${DOCKER_NETWORK:-}"
if [ "$CURL_VIA_DOCKER" = "1" ] && [ -z "$DOCKER_NETWORK" ]; then
  DOCKER_NETWORK=$(docker inspect lianel-comp-ai-service --format '{{range .NetworkSettings.Networks}}{{.NetworkID}}{{end}}' 2>/dev/null | head -1)
fi

# Run curl; if CURL_VIA_DOCKER=1 and DOCKER_NETWORK set, run inside a container on that network (for comp-ai not published on host).
# Mounts /tmp so -F "file=@/tmp/..." works; ensure file is world-readable for container process.
run_curl() {
  if [ "$CURL_VIA_DOCKER" = "1" ] && [ -n "$DOCKER_NETWORK" ]; then
    docker run --rm -v /tmp:/tmp --network "$DOCKER_NETWORK" curlimages/curl:latest "$@"
  else
    curl "$@"
  fi
}

if [ -z "$CLIENT_SECRET" ]; then
  echo "Fetching client secret from Keycloak admin..."
  ADMIN_RESP=$(curl -s -X POST "${KEYCLOAK_URL}/realms/master/protocol/openid-connect/token" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "client_id=admin-cli" \
    -d "username=${KEYCLOAK_ADMIN_USER}" \
    -d "password=${KEYCLOAK_ADMIN_PASSWORD}" \
    -d "grant_type=password")
  ADMIN_TOKEN=$(echo "$ADMIN_RESP" | python3 -c "import sys, json; print(json.load(sys.stdin).get('access_token', '') or '')" 2>/dev/null)
  [ -z "$ADMIN_TOKEN" ] && { echo "ERROR: No Keycloak admin token"; exit 1; }
  CLIENTS_JSON=$(curl -s -X GET "${KEYCLOAK_URL}/admin/realms/${REALM}/clients?clientId=${CLIENT_ID}" -H "Authorization: Bearer $ADMIN_TOKEN")
  CLIENT_UUID=$(echo "$CLIENTS_JSON" | python3 -c "import sys, json; c=json.load(sys.stdin); print(c[0]['id'] if c else '')" 2>/dev/null)
  SECRET_JSON=$(curl -s -X GET "${KEYCLOAK_URL}/admin/realms/${REALM}/clients/${CLIENT_UUID}/client-secret" -H "Authorization: Bearer $ADMIN_TOKEN")
  CLIENT_SECRET=$(echo "$SECRET_JSON" | python3 -c "import sys, json; print(json.load(sys.stdin).get('value', '') or '')" 2>/dev/null)
fi
[ -z "$CLIENT_SECRET" ] && { echo "ERROR: COMP_AI_KEYCLOAK_CLIENT_SECRET not set and could not fetch"; exit 1; }

TOKEN_RESP=$(curl -s -X POST "${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials" \
  -d "client_id=${CLIENT_ID}" \
  -d "client_secret=${CLIENT_SECRET}")
TOKEN=$(echo "$TOKEN_RESP" | python3 -c "import sys, json; print(json.load(sys.stdin).get('access_token', '') or '')" 2>/dev/null)
[ -z "$TOKEN" ] && { echo "ERROR: No access token"; exit 1; }

echo "=== 1. GET /api/v1/controls ==="
CONTROLS=$(run_curl -s -H "Authorization: Bearer $TOKEN" "${BASE_URL}/api/v1/controls")
echo "$CONTROLS" | python3 -c "import sys, json; d=json.load(sys.stdin); print('Count:', len(d) if isinstance(d, list) else 'N/A'); c=d[0] if isinstance(d, list) and d else {}; print('First control id:', c.get('id'))" 2>/dev/null || echo "$CONTROLS"
CONTROL_ID=$(echo "$CONTROLS" | python3 -c "import sys, json; d=json.load(sys.stdin); print(d[0]['id'] if isinstance(d, list) and d else '')" 2>/dev/null)
[ -z "$CONTROL_ID" ] && { echo "No control_id for upload test"; CONTROL_ID=1; }

echo ""
echo "=== 2. POST /api/v1/evidence/upload (small txt file) ==="
# Use /tmp so when CURL_VIA_DOCKER=1 the file is visible in the container ( -v /tmp:/tmp ).
TMPFILE=$(TMPDIR=/tmp mktemp)
echo "Phase B test file. Minimal content for extraction." > "$TMPFILE"
chmod 644 "$TMPFILE"
UPLOAD_RESP=$(run_curl -s -w "\n%{http_code}" -X POST "${BASE_URL}/api/v1/evidence/upload" \
  -H "Authorization: Bearer $TOKEN" \
  -F "control_id=${CONTROL_ID}" \
  -F "type=document" \
  -F "file=@${TMPFILE};type=text/plain")
rm -f "$TMPFILE"
UPLOAD_BODY=$(echo "$UPLOAD_RESP" | head -n -1)
UPLOAD_CODE=$(echo "$UPLOAD_RESP" | tail -1)
echo "HTTP $UPLOAD_CODE"
echo "$UPLOAD_BODY" | python3 -m json.tool 2>/dev/null || echo "$UPLOAD_BODY"
EVIDENCE_ID=$(echo "$UPLOAD_BODY" | python3 -c "import sys, json; d=json.load(sys.stdin); print(d.get('id', ''))" 2>/dev/null)

if [ -n "$EVIDENCE_ID" ] && [ "$UPLOAD_CODE" = "201" ]; then
  echo ""
  echo "=== 3. POST /api/v1/evidence/${EVIDENCE_ID}/analyze ==="
  ANALYSE_RESP=$(run_curl -s -w "\n%{http_code}" -X POST "${BASE_URL}/api/v1/evidence/${EVIDENCE_ID}/analyze" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json")
  ANALYSE_BODY=$(echo "$ANALYSE_RESP" | head -n -1)
  ANALYSE_CODE=$(echo "$ANALYSE_RESP" | tail -1)
  echo "HTTP $ANALYSE_CODE"
  echo "$ANALYSE_BODY" | python3 -m json.tool 2>/dev/null || echo "$ANALYSE_BODY"
else
  echo "Skipping analyse (upload returned $UPLOAD_CODE or no id). If 503, set COMP_AI_EVIDENCE_STORAGE_PATH on the comp-ai container."
fi

echo ""
echo "Done."
