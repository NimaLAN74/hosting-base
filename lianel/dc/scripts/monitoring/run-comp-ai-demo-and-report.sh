#!/usr/bin/env bash
# Comp-AI demo: run all controls/tests APIs and generate a client-ready report.
# Usage:
#   COMP_AI_TOKEN=<bearer_token> bash scripts/monitoring/run-comp-ai-demo-and-report.sh
#   # Or with Keycloak credentials (client must allow Direct access grants):
#   DEMO_USER=youruser DEMO_PASSWORD=yourpass COMP_AI_KEYCLOAK_CLIENT_ID=comp-ai-service \
#   COMP_AI_KEYCLOAK_CLIENT_SECRET=... KEYCLOAK_URL=https://www.lianel.se/auth bash scripts/monitoring/run-comp-ai-demo-and-report.sh
#
# Output: report in lianel/dc/docs/demo/COMP-AI-DEMO-REPORT-<date>.md and raw JSON in docs/demo/demo-output-<date>/
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DC_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$DC_DIR"

BASE_URL="${COMP_AI_BASE_URL:-https://www.lianel.se}"
OUTPUT_DIR="${COMP_AI_DEMO_OUTPUT:-docs/demo}"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
REPORT_DIR="${OUTPUT_DIR}/demo-output-${TIMESTAMP}"
REPORT_FILE="${OUTPUT_DIR}/COMP-AI-DEMO-REPORT-${TIMESTAMP}.md"
mkdir -p "$REPORT_DIR"

# Token: use COMP_AI_TOKEN or fetch via Keycloak
TOKEN="${COMP_AI_TOKEN:-}"
if [ -z "$TOKEN" ] && [ -n "${DEMO_USER:-}" ] && [ -n "${DEMO_PASSWORD:-}" ]; then
  KEYCLOAK_URL="${KEYCLOAK_URL:-${BASE_URL}/auth}"
  REALM="${KEYCLOAK_REALM:-lianel}"
  CLIENT_ID="${COMP_AI_KEYCLOAK_CLIENT_ID:-comp-ai-service}"
  CLIENT_SECRET="${COMP_AI_KEYCLOAK_CLIENT_SECRET:-}"
  echo "Fetching token from Keycloak (${KEYCLOAK_URL}, realm=${REALM})..."
  if [ -n "$CLIENT_SECRET" ]; then
    TOKEN_RESPONSE=$(curl -s -X POST "${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/token" \
      -H "Content-Type: application/x-www-form-urlencoded" \
      -d "client_id=${CLIENT_ID}" \
      -d "client_secret=${CLIENT_SECRET}" \
      -d "username=${DEMO_USER}" \
      -d "password=${DEMO_PASSWORD}" \
      -d "grant_type=password")
  else
    TOKEN_RESPONSE=$(curl -s -X POST "${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/token" \
      -H "Content-Type: application/x-www-form-urlencoded" \
      -d "client_id=${CLIENT_ID}" \
      -d "username=${DEMO_USER}" \
      -d "password=${DEMO_PASSWORD}" \
      -d "grant_type=password")
  fi
  TOKEN=$(echo "$TOKEN_RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin).get('access_token', ''))" 2>/dev/null || true)
fi

if [ -z "$TOKEN" ]; then
  echo "ERROR: No token. Set COMP_AI_TOKEN=<bearer_token> or DEMO_USER + DEMO_PASSWORD (and optionally COMP_AI_KEYCLOAK_CLIENT_ID, COMP_AI_KEYCLOAK_CLIENT_SECRET, KEYCLOAK_URL)."
  echo "Get a token: log in to ${BASE_URL}, open Comp AI, DevTools → Network → request to /api/v1/comp-ai/process → copy Authorization Bearer value."
  exit 1
fi

AUTH_HEADER="Authorization: Bearer ${TOKEN}"
# Timeouts for CI (GitHub runner may be slow to reach www): override with CURL_CONNECT_TIMEOUT / CURL_MAX_TIME
CURL_CT="${CURL_CONNECT_TIMEOUT:-30}"
CURL_MT="${CURL_MAX_TIME:-60}"

# Helper: GET and save status + body
api_get() {
  local path="$1"
  local outname="$2"
  local url="${BASE_URL}${path}"
  curl -s --connect-timeout "$CURL_CT" --max-time "$CURL_MT" -w "\n%{http_code}" -H "$AUTH_HEADER" "$url" > "${REPORT_DIR}/${outname}.raw"
  local body=$(sed '$d' "${REPORT_DIR}/${outname}.raw")
  local code=$(tail -1 "${REPORT_DIR}/${outname}.raw")
  echo "$body" > "${REPORT_DIR}/${outname}.json"
  echo "$code"
}

# Helper: POST and save status + body
api_post() {
  local path="$1"
  local body="$2"
  local outname="$3"
  local url="${BASE_URL}${path}"
  curl -s --connect-timeout "$CURL_CT" --max-time "$CURL_MT" -w "\n%{http_code}" -X POST -H "$AUTH_HEADER" -H "Content-Type: application/json" -d "$body" "$url" > "${REPORT_DIR}/${outname}.raw"
  local resp=$(sed '$d' "${REPORT_DIR}/${outname}.raw")
  local code=$(tail -1 "${REPORT_DIR}/${outname}.raw")
  echo "$resp" > "${REPORT_DIR}/${outname}.json"
  echo "$code"
}

echo "=== Comp-AI Demo: Running all controls/tests ==="
echo "Base URL: $BASE_URL"
echo "Output: $REPORT_DIR + $REPORT_FILE"
echo ""

# --- Health (no auth) ---
echo "[1/14] Health..."
HEALTH_CODE=$(curl -s --connect-timeout "$CURL_CT" --max-time "$CURL_MT" -o "${REPORT_DIR}/health.json" -w "%{http_code}" "${BASE_URL}/api/v1/comp-ai/health")
echo "  HTTP $HEALTH_CODE"

# --- Controls ---
echo "[2/14] GET /api/v1/controls..."
CONTROLS_CODE=$(api_get "/api/v1/controls" "controls")
echo "  HTTP $CONTROLS_CODE"

# --- Control IDs for detail calls ---
CONTROL_IDS=$(python3 -c "
import json
try:
    with open('${REPORT_DIR}/controls.json') as f:
        d = json.load(f)
        print(' '.join(str(c['id']) for c in d))
except Exception:
    print('')
" 2>/dev/null || echo "")

# --- Control detail (first control) ---
echo "[3/14] GET /api/v1/controls/:id (per control)..."
CONTROL_DETAIL_CODES=""
for id in $CONTROL_IDS; do
  code=$(api_get "/api/v1/controls/${id}" "control_${id}")
  CONTROL_DETAIL_CODES="${CONTROL_DETAIL_CODES} ${id}:${code}"
done
echo "  $CONTROL_DETAIL_CODES"

# --- Gaps ---
echo "[4/14] GET /api/v1/controls/gaps..."
GAPS_CODE=$(api_get "/api/v1/controls/gaps" "gaps")
echo "  HTTP $GAPS_CODE"

# --- Export JSON ---
echo "[5/14] GET /api/v1/controls/export?format=json..."
EXPORT_JSON_CODE=$(api_get "/api/v1/controls/export?format=json" "export_json")
echo "  HTTP $EXPORT_JSON_CODE"

# --- Export CSV ---
echo "[6/14] GET /api/v1/controls/export?format=csv..."
curl -s --connect-timeout "$CURL_CT" --max-time "$CURL_MT" -o "${REPORT_DIR}/export_csv.csv" -w "%{http_code}" -H "$AUTH_HEADER" "${BASE_URL}/api/v1/controls/export?format=csv" > "${REPORT_DIR}/export_csv_code.txt" || true
EXPORT_CSV_CODE=$(cat "${REPORT_DIR}/export_csv_code.txt" 2>/dev/null || echo "0")
echo "  HTTP $EXPORT_CSV_CODE"

# --- Requirements ---
echo "[7/14] GET /api/v1/requirements..."
REQ_CODE=$(api_get "/api/v1/requirements" "requirements")
echo "  HTTP $REQ_CODE"

echo "[8/14] GET /api/v1/requirements?framework=soc2..."
REQ_SOC2_CODE=$(api_get "/api/v1/requirements?framework=soc2" "requirements_soc2")
echo "  HTTP $REQ_SOC2_CODE"

# --- Remediation ---
echo "[9/14] GET /api/v1/remediation..."
REMED_CODE=$(api_get "/api/v1/remediation" "remediation")
echo "  HTTP $REMED_CODE"

# --- Evidence ---
echo "[10/14] GET /api/v1/evidence..."
EVIDENCE_CODE=$(api_get "/api/v1/evidence" "evidence")
echo "  HTTP $EVIDENCE_CODE"

# --- Tests ---
echo "[11/14] GET /api/v1/tests..."
TESTS_CODE=$(api_get "/api/v1/tests" "tests")
echo "  HTTP $TESTS_CODE"

# --- Control tests (per control) ---
echo "[12/14] GET /api/v1/controls/:id/tests..."
for id in $CONTROL_IDS; do
  api_get "/api/v1/controls/${id}/tests" "control_${id}_tests" >/dev/null
done
echo "  done for ${CONTROL_IDS:-no controls}"

# --- Remediation suggest (first control) ---
FIRST_ID=$(echo "$CONTROL_IDS" | awk '{print $1}')
echo "[13/14] POST /api/v1/controls/:id/remediation/suggest (control_id=${FIRST_ID:-none})..."
if [ -n "$FIRST_ID" ]; then
  SUGGEST_CODE=$(api_post "/api/v1/controls/${FIRST_ID}/remediation/suggest" '{"context":"Demo run for client report"}' "remediation_suggest")
  echo "  HTTP $SUGGEST_CODE"
else
  echo "  skip (no control id)"
fi

# --- Frameworks (comp-ai path) ---
echo "[14/14] GET /api/v1/comp-ai/frameworks..."
FRAMEWORKS_CODE=$(api_get "/api/v1/comp-ai/frameworks" "frameworks")
echo "  HTTP $FRAMEWORKS_CODE"

# --- Generate Markdown report ---
echo ""
echo "Writing report to $REPORT_FILE ..."

python3 << PYEOF
import json
import os
from datetime import datetime

report_dir = "${REPORT_DIR}"
report_file = "${REPORT_FILE}"
base_url = "${BASE_URL}"
ts = "${TIMESTAMP}"

def read_json(name, default=None):
    p = os.path.join(report_dir, name + ".json")
    if not os.path.isfile(p):
        return default
    try:
        with open(p) as f:
            return json.load(f)
    except Exception:
        with open(p) as f:
            return f.read()

def read_file(name):
    p = os.path.join(report_dir, name)
    if os.path.isfile(p):
        with open(p) as f:
            return f.read()
    return ""

lines = []
def out(s=""):
    lines.append(s)

out("# Comp-AI Demo Report")
out("")
out(f"**Generated**: {datetime.utcnow().strftime('%d/%m/%Y %H:%M UTC')}  ")
out(f"**Environment**: {base_url}  ")
out("")
out("---")
out("")

# Summary table
out("## 1. Summary")
out("")
out("| Endpoint | Status | Notes |")
out("|----------|--------|-------|")

health_body = read_file("health.json")
health_ok = "ok" in health_body.lower() if health_body else False
out(f"| GET /api/v1/comp-ai/health | {'✅ 200' if health_ok else '❌'} | Public health check |")

codes = {
    "controls": read_file("controls.json") and "200" or "—",
    "gaps": read_file("gaps.json") and "200" or "—",
    "export_json": read_file("export_json.json") and "200" or "—",
    "requirements": read_file("requirements.json") and "200" or "—",
    "remediation": read_file("remediation.json") and "200" or "—",
    "evidence": read_file("evidence.json") and "200" or "—",
    "tests": read_file("tests.json") and "200" or "—",
    "frameworks": read_file("frameworks.json") and "200" or "—",
}
# We don't have HTTP codes stored per endpoint; use presence of JSON
for label, path in [
    ("GET /api/v1/controls", "controls"),
    ("GET /api/v1/controls/gaps", "gaps"),
    ("GET /api/v1/controls/export (JSON)", "export_json"),
    ("GET /api/v1/requirements", "requirements"),
    ("GET /api/v1/remediation", "remediation"),
    ("GET /api/v1/evidence", "evidence"),
    ("GET /api/v1/tests", "tests"),
    ("GET /api/v1/comp-ai/frameworks", "frameworks"),
]:
    body = read_file(path + ".json")
    ok = body and "error" not in body[:200].lower()
    out(f"| {label} | {'✅ 200' if ok else '—'} | |")

out("")
out("---")
out("")

# Controls
out("## 2. Controls")
controls = read_json("controls")
if isinstance(controls, list) and controls:
    out(f"Total controls: **{len(controls)}**")
    out("")
    out("| Internal ID | Name | Category |")
    out("|-------------|------|----------|")
    for c in controls:
        out(f"| {c.get('internal_id', '')} | {c.get('name', '')} | {c.get('category') or '—'} |")
else:
    out("*No controls or invalid response.*")
out("")
out("---")
out("")

# Gaps
out("## 3. Controls without evidence (gaps)")
gaps = read_json("gaps")
if isinstance(gaps, list):
    out(f"Count: **{len(gaps)}**")
    if gaps:
        out("")
        for g in gaps[:20]:
            out(f"- {g.get('internal_id', '')}: {g.get('name', '')}")
        if len(gaps) > 20:
            out(f"- ... and {len(gaps) - 20} more")
else:
    out("*No data or invalid response.*")
out("")
out("---")
out("")

# Requirements
out("## 4. Framework requirements (sample)")
req = read_json("requirements")
if isinstance(req, list) and req:
    out(f"Total (all frameworks): **{len(req)}**")
    out("")
    out("| Code | Title | Framework |")
    out("|------|-------|-----------|")
    for r in req[:15]:
        out(f"| {r.get('code', '')} | {(r.get('title') or '')[:50]} | {r.get('framework_slug', '')} |")
else:
    out("*No requirements or invalid response.*")
out("")
out("---")
out("")

# Remediation
out("## 5. Remediation tasks")
remed = read_json("remediation")
if isinstance(remed, list) and remed:
    out(f"Total: **{len(remed)}**")
    for t in remed[:10]:
        out(f"- Control {t.get('control_id')}: status={t.get('status')}, assigned_to={t.get('assigned_to') or '—'}")
else:
    out("*No remediation tasks or invalid response.*")
out("")
out("---")
out("")

# Tests
out("## 6. Automated tests per control")
tests = read_json("tests")
if isinstance(tests, list) and tests:
    out(f"Total test definitions: **{len(tests)}**")
    out("")
    out("| Control ID | Test name | Type | Last result |")
    out("|------------|-----------|------|-------------|")
    for t in tests[:20]:
        out(f"| {t.get('control_id')} | {t.get('name', '')} | {t.get('test_type', '')} | {t.get('last_result') or '—'} |")
else:
    out("*No tests or invalid response.*")
out("")
out("---")
out("")

# Remediation suggest (sample)
out("## 7. AI remediation suggestion (sample)")
suggest = read_json("remediation_suggest")
if suggest and isinstance(suggest, dict):
    out("**Model used**: " + suggest.get("model_used", "—"))
    out("")
    out("**Suggestion**:")
    out("")
    out("```")
    out((suggest.get("suggestion") or "")[:2000])
    out("```")
else:
    out("*No suggestion or endpoint not available (e.g. Ollama not configured).*")
out("")
out("---")
out("")

# Export
out("## 8. Audit export")
out("CSV and JSON export are saved in this run's output directory for audit use.")
out("")
out("---")
out("")
out("*End of report. Raw JSON/CSV files: " + report_dir + "*")

with open(report_file, "w") as f:
    f.write("\n".join(lines))
PYEOF

echo "Done. Report: $REPORT_FILE"
echo "Raw output: $REPORT_DIR"
