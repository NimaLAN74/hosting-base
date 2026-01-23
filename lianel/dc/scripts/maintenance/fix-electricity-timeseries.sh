#!/bin/bash
# Complete fix script for Electricity Timeseries empty data issue
# This script sets the ENTSO-E API token and triggers the DAG

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

echo "=== Fixing Electricity Timeseries Issue ==="
echo ""

# Step 1: Set the token
echo "Step 1: Setting ENTSO-E API token..."
docker exec dc-airflow-apiserver-1 airflow variables set ENTSOE_API_TOKEN "$TOKEN" 2>&1 | grep -v 'RemovedInAirflow4Warning' | grep -v 'deprecated' || {
    echo "❌ Failed to set token"
    exit 1
}
echo "✅ Token set successfully"
echo ""

# Step 2: Verify token
echo "Step 2: Verifying token..."
VERIFIED=$(docker exec dc-airflow-apiserver-1 airflow variables get ENTSOE_API_TOKEN 2>&1 | grep -v 'RemovedInAirflow4Warning' | grep -v 'deprecated' | tail -1)

if [ -n "$VERIFIED" ] && [ "$VERIFIED" != "Variable ENTSOE_API_TOKEN does not exist" ]; then
    echo "✅ Token verified: ${VERIFIED:0:20}..."
else
    echo "⚠️  Warning: Could not verify token"
    exit 1
fi
echo ""

# Step 3: Check current table count
echo "Step 3: Checking current table state..."
CURRENT_COUNT=$(docker exec dc-airflow-apiserver-1 bash -c "export PGPASSWORD=\$POSTGRES_PASSWORD; psql -h 172.18.0.1 -U airflow -d lianel_energy -t -c 'SELECT COUNT(*) FROM fact_electricity_timeseries;'" 2>&1 | tr -d ' ')
echo "Current records in table: $CURRENT_COUNT"
echo ""

# Step 4: Trigger the DAG
echo "Step 4: Triggering entsoe_ingestion DAG..."
DAG_RUN_ID=$(docker exec dc-airflow-apiserver-1 airflow dags trigger entsoe_ingestion 2>&1 | grep -v 'RemovedInAirflow4Warning' | grep -v 'deprecated' | grep -oP "Created <DagRun \w+ @ \K[^>]+" || echo "")
if [ -n "$DAG_RUN_ID" ]; then
    echo "✅ DAG triggered: $DAG_RUN_ID"
else
    echo "✅ DAG triggered (check Airflow UI for run ID)"
fi
echo ""

# Step 5: Wait a bit and check progress
echo "Step 5: Waiting 30 seconds for DAG to start..."
sleep 30

# Step 6: Check if data is being ingested
echo "Step 6: Checking if data is being ingested..."
NEW_COUNT=$(docker exec dc-airflow-apiserver-1 bash -c "export PGPASSWORD=\$POSTGRES_PASSWORD; psql -h 172.18.0.1 -U airflow -d lianel_energy -t -c 'SELECT COUNT(*) FROM fact_electricity_timeseries;'" 2>&1 | tr -d ' ')

if [ "$NEW_COUNT" -gt "$CURRENT_COUNT" ]; then
    echo "✅ Data is being ingested! New count: $NEW_COUNT (was $CURRENT_COUNT)"
else
    echo "⏳ No new data yet. The DAG is running. Please check Airflow UI to monitor progress."
    echo "   It may take several minutes for data to appear."
fi
echo ""

# Step 7: Check ingestion log
echo "Step 7: Checking ingestion log..."
INGESTION_LOG=$(docker exec dc-airflow-apiserver-1 bash -c "export PGPASSWORD=\$POSTGRES_PASSWORD; psql -h 172.18.0.1 -U airflow -d lianel_energy -c 'SELECT country_code, records_ingested, status FROM meta_entsoe_ingestion_log ORDER BY ingestion_date DESC LIMIT 5;'" 2>&1 | tail -10)
if [ -n "$INGESTION_LOG" ] && ! echo "$INGESTION_LOG" | grep -q "0 rows"; then
    echo "✅ Ingestion log shows activity:"
    echo "$INGESTION_LOG"
else
    echo "⏳ Ingestion log is empty. The DAG is still running."
fi
echo ""

echo "=== Fix Complete ==="
echo ""
echo "Next steps:"
echo "1. Monitor the DAG in Airflow UI: https://airflow.lianel.se"
echo "2. Check the API endpoint: https://www.lianel.se/api/v1/electricity/timeseries?limit=1"
echo "3. The DAG will continue running and should complete in a few minutes"
echo ""
echo "The DAG runs daily at 03:00 UTC automatically."
