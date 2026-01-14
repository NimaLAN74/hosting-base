#!/bin/bash
# DAG Performance Analysis Script
# Analyzes Airflow DAG execution times and task durations

set -e

echo "=== Airflow DAG Performance Analysis ==="
echo ""

# Check if we're in a container or need to use docker exec
if command -v airflow &> /dev/null; then
    AIRFLOW_CMD="airflow"
else
    # Try to find Airflow container
    AIRFLOW_CONTAINER=$(docker ps --filter 'name=airflow' --format '{{.Names}}' | grep -E 'scheduler|webserver' | head -1)
    if [ -z "$AIRFLOW_CONTAINER" ]; then
        echo "Error: Airflow container not found"
        exit 1
    fi
    echo "Note: Running via docker exec ($AIRFLOW_CONTAINER)"
    AIRFLOW_CMD="docker exec $AIRFLOW_CONTAINER airflow"
fi

echo "=== DAG List ==="
$AIRFLOW_CMD dags list 2>&1 | grep -E 'ml_dataset|entsoe|osm|nuts' | head -20
echo ""

echo "=== Recent DAG Runs (Last 10) ==="
for dag_id in ml_dataset_forecasting_dag ml_dataset_clustering_dag ml_dataset_geo_enrichment_dag entsoe_ingestion osm_feature_extraction_dag; do
    echo ""
    echo "--- $dag_id ---"
    $AIRFLOW_CMD dags list-runs --dag-id "$dag_id" --limit 5 2>&1 | head -10 || echo "DAG not found or no runs"
done

echo ""
echo "=== DAG Execution Summary ==="
echo "To get detailed task execution times, use:"
echo "  airflow tasks list <dag_id>"
echo "  airflow tasks state <dag_id> <task_id> <execution_date>"
echo ""
echo "=== Analysis Complete ==="
