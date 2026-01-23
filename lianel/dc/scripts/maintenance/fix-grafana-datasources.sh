#!/bin/bash
# Fix Grafana datasources by substituting password and fixing URLs
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DATASOURCES_FILE="${PROJECT_DIR}/monitoring/grafana/provisioning/datasources/datasources.yml"

echo "=== Fixing Grafana Datasources ==="

# Get POSTGRES_PASSWORD from Airflow container or .env
if [ -f "${PROJECT_DIR}/.env" ]; then
    export $(grep -v '^#' "${PROJECT_DIR}/.env" | grep POSTGRES_PASSWORD | xargs)
fi

if [ -z "$POSTGRES_PASSWORD" ]; then
    # Try to get from Airflow container
    POSTGRES_PASSWORD=$(docker exec dc-airflow-apiserver-1 env 2>/dev/null | grep POSTGRES_PASSWORD | cut -d'=' -f2 || echo "")
fi

if [ -z "$POSTGRES_PASSWORD" ]; then
    echo "Error: POSTGRES_PASSWORD not found"
    exit 1
fi

echo "✅ Found POSTGRES_PASSWORD (length: ${#POSTGRES_PASSWORD})"

# Backup original file
if [ -f "$DATASOURCES_FILE" ]; then
    cp "$DATASOURCES_FILE" "${DATASOURCES_FILE}.bak"
    echo "✅ Created backup: ${DATASOURCES_FILE}.bak"
fi

# Substitute password and fix URLs using sed
sed -i.tmp \
    -e "s|\${POSTGRES_PASSWORD}|${POSTGRES_PASSWORD}|g" \
    -e "s|url: postgres:5432|url: 172.18.0.1:5432|g" \
    "$DATASOURCES_FILE"

rm -f "${DATASOURCES_FILE}.tmp"

echo "✅ Substituted password and fixed URLs in datasources.yml"
echo "✅ File ready for deployment"

# Verify changes
echo ""
echo "Verification:"
grep -A 2 "secureJsonData:" "$DATASOURCES_FILE" | grep "password:" | head -1
grep "url:" "$DATASOURCES_FILE" | grep "172.18.0.1:5432" | head -2

echo ""
echo "✅ Datasources file fixed successfully"
