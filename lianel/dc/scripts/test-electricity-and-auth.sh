#!/bin/bash
# Comprehensive test script for Electricity Timeseries and Monitoring page authentication

set -e

REPO_DIR="${REPO_DIR:-/root/hosting-base/lianel/dc}"

echo "=== Testing Electricity Timeseries Data ==="
echo ""

# Check if table exists
echo "1. Checking if fact_electricity_timeseries table exists..."
docker exec lianel-energy-service sh -c 'PGPASSWORD=$POSTGRES_PASSWORD psql -h 172.18.0.1 -p 5432 -U airflow -d lianel_energy -c "\d fact_electricity_timeseries" 2>&1' || echo "Table might not exist"

# Check record count
echo ""
echo "2. Checking record count..."
docker exec lianel-energy-service sh -c 'PGPASSWORD=$POSTGRES_PASSWORD psql -h 172.18.0.1 -p 5432 -U airflow -d lianel_energy -c "SELECT COUNT(*) as total FROM fact_electricity_timeseries;" 2>&1' || echo "Query failed"

# Check ingestion log
echo ""
echo "3. Checking ingestion log..."
docker exec lianel-energy-service sh -c 'PGPASSWORD=$POSTGRES_PASSWORD psql -h 172.18.0.1 -p 5432 -U airflow -d lianel_energy -c "SELECT country_code, records_ingested, status, ingestion_date FROM meta_entsoe_ingestion_log ORDER BY ingestion_date DESC LIMIT 10;" 2>&1' || echo "Query failed"

# Test API endpoint
echo ""
echo "4. Testing API endpoint..."
curl -s "https://www.lianel.se/api/v1/electricity/timeseries?limit=5" | python3 -m json.tool 2>&1 | head -20 || echo "API request failed"

# Check DAG status
echo ""
echo "5. Checking latest DAG run status..."
docker exec dc-airflow-apiserver-1 airflow dags list-runs --dag-id entsoe_ingestion --limit 1 2>&1 | grep -E '(state|run_id|start_date)' | head -5 || echo "DAG status check failed"

echo ""
echo "=== Testing Monitoring Page Authentication ==="
echo ""

# Check frontend container
echo "6. Checking frontend container status..."
docker ps --filter "name=lianel-frontend" --format "{{.Status}}\t{{.Image}}" || echo "Frontend container not found"

# Check frontend logs for errors
echo ""
echo "7. Checking frontend logs for authentication errors..."
docker logs lianel-frontend --tail 20 2>&1 | grep -E '(error|Error|ERROR|auth|keycloak|login)' || echo "No errors found in recent logs"

echo ""
echo "=== Test Complete ==="
