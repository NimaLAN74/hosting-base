#!/bin/bash
# Trace the full Airflow + Keycloak flow: follow redirects and capture each step.
# Run: bash scripts/monitoring/e2e-airflow-keycloak-flow-trace.sh
# No browser; uses curl with -v and -L to see exact redirects and responses.

set -e
AIRFLOW_URL="${AIRFLOW_URL:-https://airflow.lianel.se}"
OUTDIR="${OUTDIR:-/tmp/airflow-e2e-trace}"
mkdir -p "$OUTDIR"
COOKIE_JAR="$OUTDIR/cookies.txt"
: > "$COOKIE_JAR"

echo "=========================================="
echo "Airflow + Keycloak flow trace"
echo "=========================================="
echo "Output dir: $OUTDIR"
echo ""

# Step 1: GET login page
echo "--- Step 1: GET $AIRFLOW_URL/auth/login/ ---"
curl -sS -c "$COOKIE_JAR" -b "$COOKIE_JAR" -L -o "$OUTDIR/1_login.html" -w "HTTP %{http_code}\nRedirects: %{num_redirects}\nFinal URL: %{url_effective}\n" "$AIRFLOW_URL/auth/login/" | tee "$OUTDIR/1_headers.txt"
echo "Saved: $OUTDIR/1_login.html"
echo ""

# Step 2: GET auth/login/keycloak (do not follow - capture Location)
echo "--- Step 2: GET $AIRFLOW_URL/auth/login/keycloak ---"
REDIRECT_TO=$(curl -sS -c "$COOKIE_JAR" -b "$COOKIE_JAR" -o "$OUTDIR/2_response.html" -w "%{url_effective}" -D "$OUTDIR/2_headers.txt" "$AIRFLOW_URL/auth/login/keycloak")
HTTP_2=$(grep -E "^HTTP" "$OUTDIR/2_headers.txt" | tail -1)
LOCATION_2=$(grep -i "^Location:" "$OUTDIR/2_headers.txt" | head -1 | sed 's/^[Ll]ocation:[[:space:]]*//' | tr -d '\r\n')
echo "$HTTP_2"
echo "Location: $LOCATION_2"
echo "Saved: $OUTDIR/2_headers.txt $OUTDIR/2_response.html"
# Encoded http scheme is redirect_uri=http%3A%2F%2F (https is https%3A - don't false-positive)
if echo "$LOCATION_2" | grep -q 'redirect_uri=http%3A%2F%2F'; then
  echo ">>> ISSUE: redirect_uri is HTTP (Keycloak will 400)"
elif echo "$LOCATION_2" | grep -q 'redirect_uri=https%3A%2F%2F'; then
  echo ">>> redirect_uri is HTTPS (correct)"
fi
echo ""

# Step 3: Follow to Keycloak (GET the auth URL)
if [ -n "$LOCATION_2" ]; then
  echo "--- Step 3: GET Keycloak auth URL ---"
  curl -sS -c "$COOKIE_JAR" -b "$COOKIE_JAR" -L -o "$OUTDIR/3_keycloak.html" -w "HTTP %{http_code}\nRedirects: %{num_redirects}\nFinal URL: %{url_effective}\n" "$LOCATION_2" | tee "$OUTDIR/3_headers.txt"
  echo "Saved: $OUTDIR/3_keycloak.html"
  if grep -qi "sign in\|login\|keycloak" "$OUTDIR/3_keycloak.html" 2>/dev/null; then
    echo ">>> Keycloak login page content detected"
  fi
  if grep -qi "error\|invalid" "$OUTDIR/3_keycloak.html" 2>/dev/null; then
    echo ">>> Possible error text in body (first 5 lines):"
    head -5 "$OUTDIR/3_keycloak.html"
  fi
  echo ""
fi

# Step 4: GET callback URL without code (expect 302 to login or 400)
echo "--- Step 4: GET callback (no code) $AIRFLOW_URL/auth/oauth-authorized/keycloak ---"
curl -sS -c "$COOKIE_JAR" -b "$COOKIE_JAR" -o "$OUTDIR/4_callback.html" -w "HTTP %{http_code}\n" -D "$OUTDIR/4_headers.txt" "$AIRFLOW_URL/auth/oauth-authorized/keycloak" | tee "$OUTDIR/4_status.txt"
echo "Saved: $OUTDIR/4_callback.html $OUTDIR/4_headers.txt"
CALLBACK_HTTP=$(grep -E "^HTTP" "$OUTDIR/4_headers.txt" | tail -1)
echo "$CALLBACK_HTTP"
if grep -q "500" "$OUTDIR/4_headers.txt"; then
  echo ">>> ISSUE: Callback returned 500. Body snippet:"
  head -30 "$OUTDIR/4_callback.html"
fi
echo ""

# Step 5: GET callback with bogus code (expect 400 or redirect to login)
echo "--- Step 5: GET callback with ?code=fake&state=fake ---"
curl -sS -c "$COOKIE_JAR" -b "$COOKIE_JAR" -o "$OUTDIR/5_callback_with_code.html" -w "HTTP %{http_code}\n" -D "$OUTDIR/5_headers.txt" "$AIRFLOW_URL/auth/oauth-authorized/keycloak?code=fake&state=fake" 2>&1 | tee "$OUTDIR/5_status.txt"
echo "Saved: $OUTDIR/5_callback_with_code.html"
grep -E "^HTTP" "$OUTDIR/5_headers.txt" || true
if grep -q "500" "$OUTDIR/5_headers.txt"; then
  echo ">>> ISSUE: Callback with code returned 500. Body:"
  head -40 "$OUTDIR/5_callback_with_code.html"
fi
echo ""

echo "=========================================="
echo "Trace complete. Inspect: $OUTDIR/"
echo "=========================================="
