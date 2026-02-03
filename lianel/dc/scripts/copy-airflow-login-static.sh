#!/bin/bash
# Copy FAB appbuilder and Airflow FAB provider dist static files from the Airflow
# apiserver container to a host directory so nginx can serve them for /auth/static/
# (avoids MIME mismatch when Airflow returns HTML/JSON for unauthenticated requests).
#
# Run on the server. Requires: Airflow apiserver container running.
# Usage: AIRFLOW_LOGIN_STATIC_DIR=/root/airflow-login-static AIRFLOW_CONTAINER=dc-airflow-apiserver-1 ./copy-airflow-login-static.sh

set -e

HOST_DIR="${AIRFLOW_LOGIN_STATIC_DIR:-/root/airflow-login-static}"
CONTAINER="${AIRFLOW_CONTAINER:-dc-airflow-apiserver-1}"

FAB_APPBUILDER="/home/airflow/.local/lib/python3.12/site-packages/flask_appbuilder/static/appbuilder"
FAB_DIST="/home/airflow/.local/lib/python3.12/site-packages/airflow/providers/fab/www/static/dist"

echo "=== Copying Airflow login static files to $HOST_DIR ==="
echo "Container: $CONTAINER"
echo

if ! docker ps --format '{{.Names}}' | grep -q "^${CONTAINER}$"; then
  echo "ERROR: Container $CONTAINER is not running."
  exit 1
fi

mkdir -p "$HOST_DIR/appbuilder" "$HOST_DIR/dist"
docker cp "$CONTAINER:$FAB_APPBUILDER/." "$HOST_DIR/appbuilder/"
docker cp "$CONTAINER:$FAB_DIST/." "$HOST_DIR/dist/"

echo "✓ Copied appbuilder static to $HOST_DIR/appbuilder/"
echo "✓ Copied dist static to $HOST_DIR/dist/"
echo
echo "Ensure nginx has this volume: - $HOST_DIR:/var/www/airflow-login-static:ro"
echo "Then reload nginx."
