#!/bin/bash
# Script to trigger the ENTSO-E ingestion DAG on remote host
# Usage: Run this script on the remote host or via SSH

set -e

REPO_DIR="${REPO_DIR:-/root/hosting-base}"
AIRFLOW_CONTAINER="${AIRFLOW_CONTAINER:-dc-airflow-apiserver-1}"
DAG_ID="${DAG_ID:-entsoe_ingestion}"

echo "=== Triggering ENTSO-E Ingestion DAG (Remote) ==="
echo "Repository: $REPO_DIR"
echo "DAG ID: $DAG_ID"
echo "Airflow Container: $AIRFLOW_CONTAINER"
echo ""

# Navigate to repo directory
cd "$REPO_DIR" || exit 1

# Pull latest code
echo "Pulling latest code..."
git pull origin master || git pull origin main || echo "Warning: Could not pull latest code"

# Check if container is running
if ! docker ps | grep -q "$AIRFLOW_CONTAINER"; then
    echo "ERROR: Airflow container '$AIRFLOW_CONTAINER' is not running"
    echo "Available containers:"
    docker ps --format "table {{.Names}}\t{{.Status}}"
    exit 1
fi

echo "Triggering DAG: $DAG_ID"
echo "This will fetch 2 years of historical electricity data for all 23 countries..."
echo ""

# Trigger the DAG
docker exec "$AIRFLOW_CONTAINER" airflow dags trigger "$DAG_ID"

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ DAG triggered successfully!"
    echo ""
    echo "The DAG will fetch historical data for:"
    echo "  AT, BE, BG, CZ, DE, DK, EE, ES, FI, FR, HR, HU, IE, IT, LT, LU, LV, NL, PL, PT, RO, SE, SI, SK"
    echo ""
    echo "Date range: Last 2 years (730 days) from yesterday"
    echo ""
    echo "Monitor progress:"
    echo "  docker exec $AIRFLOW_CONTAINER airflow dags list-runs -d $DAG_ID --state running"
    echo ""
    echo "Or check Airflow UI:"
    echo "  https://www.lianel.se/airflow/dags/$DAG_ID/grid"
    echo ""
    echo "⚠️  This will take several hours to complete (processing 23 countries × ~104 weeks)"
else
    echo ""
    echo "❌ Failed to trigger DAG"
    echo "Check Airflow logs:"
    echo "  docker logs $AIRFLOW_CONTAINER"
    exit 1
fi
