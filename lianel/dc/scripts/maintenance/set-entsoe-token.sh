#!/bin/bash
# Script to set ENTSO-E API token in Airflow Variables
# Usage: ./set-entsoe-token.sh <token>

set -e

TOKEN="${1:-}"

if [ -z "$TOKEN" ]; then
    echo "❌ Error: ENTSO-E API token is required"
    echo "Usage: $0 <token>"
    echo ""
    echo "To get a token:"
    echo "1. Register at https://transparency.entsoe.eu/"
    echo "2. Request API access"
    echo "3. Get your API token"
    exit 1
fi

echo "=== Setting ENTSO-E API Token ==="
echo ""

# Set the token in Airflow
docker exec dc-airflow-apiserver-1 airflow variables set ENTSOE_API_TOKEN "$TOKEN" 2>&1 | grep -v 'RemovedInAirflow4Warning' | grep -v 'deprecated' || {
    echo "❌ Failed to set token"
    exit 1
}

echo "✅ Token set successfully"
echo ""

# Verify token is set
echo "Verifying token..."
VERIFIED=$(docker exec dc-airflow-apiserver-1 airflow variables get ENTSOE_API_TOKEN 2>&1 | grep -v 'RemovedInAirflow4Warning' | grep -v 'deprecated' | tail -1)

if [ -n "$VERIFIED" ] && [ "$VERIFIED" != "Variable ENTSOE_API_TOKEN does not exist" ]; then
    echo "✅ Token verified: ${VERIFIED:0:20}..."
    echo ""
    echo "Next steps:"
    echo "1. Trigger the entsoe_ingestion DAG in Airflow"
    echo "2. Monitor the DAG run to verify data is being ingested"
    echo "3. Check the API endpoint: https://www.lianel.se/api/v1/electricity/timeseries?limit=1"
else
    echo "⚠️  Warning: Could not verify token"
fi
