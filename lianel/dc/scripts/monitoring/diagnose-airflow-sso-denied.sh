#!/bin/bash
# Diagnose "The request to sign in was denied" after Keycloak login.
# Run on the server (where Airflow runs). Checks client secret and suggests viewing apiserver logs.
# Usage: bash scripts/monitoring/diagnose-airflow-sso-denied.sh

set -e

echo "=========================================="
echo "Diagnose: Airflow SSO 'sign in denied'"
echo "=========================================="
echo ""

# Find airflow apiserver container
CONTAINER=""
if [ -n "$AIRFLOW_APISERVER_CONTAINER" ]; then
  CONTAINER="$AIRFLOW_APISERVER_CONTAINER"
elif command -v docker &>/dev/null; then
  CONTAINER=$(docker ps --format '{{.Names}}' | grep -E 'airflow.*apiserver|apiserver.*airflow' | head -1)
fi
if [ -z "$CONTAINER" ]; then
  echo "Could not find Airflow apiserver container."
  echo "Set AIRFLOW_APISERVER_CONTAINER or run from the host where 'docker ps' shows the apiserver."
  exit 1
fi
echo "Using container: $CONTAINER"
echo ""

# Check AIRFLOW_OAUTH_CLIENT_SECRET (do not print value)
SECRET_SET=$(docker exec "$CONTAINER" env | grep -E '^AIRFLOW_OAUTH_CLIENT_SECRET=' || true)
if [ -z "$SECRET_SET" ]; then
  echo "❌ AIRFLOW_OAUTH_CLIENT_SECRET is not set in the container."
  echo "   Fix: Add it to .env (same dir as docker-compose.airflow.yaml), then:"
  echo "   docker compose -f docker-compose.airflow.yaml up -d --force-recreate"
  echo "   Get the value by running: bash scripts/keycloak-setup/create-airflow-keycloak-client.sh"
else
  VAL="${SECRET_SET#AIRFLOW_OAUTH_CLIENT_SECRET=}"
  if [ -z "$VAL" ]; then
    echo "❌ AIRFLOW_OAUTH_CLIENT_SECRET is set but empty."
    echo "   Fix: Set it in .env to the value from create-airflow-keycloak-client.sh and restart."
  else
    echo "✅ AIRFLOW_OAUTH_CLIENT_SECRET is set (length ${#VAL} chars)."
  fi
fi
echo ""

echo "To see the real error from the token exchange:"
echo "  1. Reproduce: open Airflow → Login → Sign in with Keycloak → log in on Keycloak."
echo "  2. When 'denied' appears, run:"
echo "     docker logs $CONTAINER 2>&1 | tail -200 | grep -A5 -i 'Error authorizing OAuth access token\\|oauth\\|token\\|denied'"
echo ""
echo "The log line 'Error authorizing OAuth access token: ...' shows the actual exception (e.g. 401, invalid_client)."
echo "See: lianel/dc/docs/fixes/KEYCLOAK-AIRFLOW-SSO-RESEARCH.md §8"
echo ""
