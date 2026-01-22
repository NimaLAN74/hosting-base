#!/bin/bash
# Script to trigger the ENTSO-E ingestion DAG
# This will fetch historical electricity data for all configured countries

set -e

AIRFLOW_CONTAINER="${AIRFLOW_CONTAINER:-dc-airflow-apiserver-1}"
DAG_ID="${DAG_ID:-entsoe_ingestion}"

echo "=== Triggering ENTSO-E Ingestion DAG ==="
echo "DAG ID: $DAG_ID"
echo "Airflow Container: $AIRFLOW_CONTAINER"
echo ""

# Check if container is running
if ! docker ps | grep -q "$AIRFLOW_CONTAINER"; then
    echo "ERROR: Airflow container '$AIRFLOW_CONTAINER' is not running"
    echo "Available containers:"
    docker ps --format "table {{.Names}}\t{{.Status}}"
    exit 1
fi

echo "Triggering DAG: $DAG_ID"
echo "This will fetch historical electricity data for all configured countries..."
echo ""

# Trigger the DAG
docker exec "$AIRFLOW_CONTAINER" airflow dags trigger "$DAG_ID"

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ DAG triggered successfully!"
    echo ""
    echo "You can monitor the progress at:"
    echo "  https://www.lianel.se/airflow/dags/$DAG_ID/grid"
    echo ""
    echo "Or check the status with:"
    echo "  docker exec $AIRFLOW_CONTAINER airflow dags list-runs -d $DAG_ID --state running"
    echo ""
    echo "The DAG will:"
    echo "  - Check last ingestion date for each country"
    echo "  - Plan date ranges to fetch (from last date to yesterday)"
    echo "  - Ingest data in batches for all 23 countries:"
    echo "    AT, BE, BG, CZ, DE, DK, EE, ES, FI, FR, HR, HU, IE, IT, LT, LU, LV, NL, PL, PT, RO, SE, SI, SK"
    echo ""
    echo "This may take several hours depending on the amount of data to fetch."
else
    echo ""
    echo "❌ Failed to trigger DAG"
    echo "Check Airflow logs:"
    echo "  docker logs $AIRFLOW_CONTAINER"
    exit 1
fi
