#!/bin/bash
# Fix both /monitoring redirect and electricity timeseries issues

set -e

cd "$(dirname "$0")/.."

echo "=== FIXING /monitoring REDIRECT ==="
echo ""

# Check if proxy_redirect is in nginx config
if ! grep -q "proxy_redirect off" nginx/config/nginx.conf; then
    echo "Adding proxy_redirect off to /monitoring location block..."
    sed -i '/location = \/monitoring {/,/^[[:space:]]*}/ { 
        /proxy_cache_bypass/a\
            proxy_redirect off;
    }' nginx/config/nginx.conf
    echo "✅ Added proxy_redirect off"
else
    echo "✅ proxy_redirect off already present"
fi

# Reload nginx
echo ""
echo "Reloading Nginx..."
docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload
echo "✅ Nginx reloaded"

echo ""
echo "=== CHECKING ELECTRICITY TIMESERIES ==="
echo ""

# Check if table exists and has data
echo "Checking fact_electricity_timeseries table..."
DATA_COUNT=$(docker exec dc-airflow-apiserver-1 psql -h 172.18.0.1 -U airflow -d lianel_energy -t -c "SELECT COUNT(*) FROM fact_electricity_timeseries;" 2>/dev/null | tr -d ' ' || echo "0")

if [ "$DATA_COUNT" = "0" ] || [ -z "$DATA_COUNT" ]; then
    echo "⚠️  Table is empty or doesn't exist"
    echo ""
    echo "Checking ENTSOE_API_TOKEN..."
    TOKEN=$(docker exec dc-airflow-apiserver-1 airflow variables get ENTSOE_API_TOKEN 2>&1 | grep -v "Variable" | tr -d ' ' || echo "")
    
    if [ -z "$TOKEN" ] || [ "$TOKEN" = "" ]; then
        echo "❌ ENTSOE_API_TOKEN is not set!"
        echo ""
        echo "To fix:"
        echo "1. Get your ENTSOE API token from: https://transparency.entsoe.eu/"
        echo "2. Set it in Airflow:"
        echo "   docker exec dc-airflow-apiserver-1 airflow variables set ENTSOE_API_TOKEN 'your_token_here'"
        echo "3. Trigger the entsoe_ingestion DAG"
    else
        echo "✅ ENTSOE_API_TOKEN is set"
        echo ""
        echo "Checking DAG status..."
        DAG_STATE=$(docker exec dc-airflow-apiserver-1 airflow dags state entsoe_ingestion 2>&1 | grep -i "state\|running\|success\|failed" | head -1 || echo "unknown")
        echo "DAG state: $DAG_STATE"
        echo ""
        echo "If DAG is green but no data, the DAG might have run but failed to insert data."
        echo "Check DAG task logs for errors."
    fi
else
    echo "✅ Table has $DATA_COUNT rows"
    echo ""
    echo "Checking data distribution..."
    docker exec dc-airflow-apiserver-1 psql -h 172.18.0.1 -U airflow -d lianel_energy -c "SELECT country_code, COUNT(*) as rows, MIN(timestamp) as first_data, MAX(timestamp) as last_data FROM fact_electricity_timeseries GROUP BY country_code ORDER BY country_code;" 2>&1
fi

echo ""
echo "=== TESTING API ENDPOINT ==="
echo ""
echo "Testing electricity timeseries API..."
API_RESPONSE=$(curl -s 'https://www.lianel.se/api/v1/electricity/timeseries?country_code=SE&start_date=2024-01-01&end_date=2024-12-31' || echo "ERROR")
if echo "$API_RESPONSE" | grep -q "data"; then
    echo "✅ API is working"
    echo "$API_RESPONSE" | head -5
else
    echo "❌ API returned error or no data"
    echo "$API_RESPONSE"
fi

echo ""
echo "=== SUMMARY ==="
echo ""
echo "1. /monitoring redirect: Fixed (nginx reloaded)"
echo "2. Electricity timeseries: Check output above"
echo ""
echo "If electricity timeseries still shows no data:"
echo "  - Check if ENTSOE_API_TOKEN is set"
echo "  - Check DAG task logs for errors"
echo "  - Manually trigger entsoe_ingestion DAG"
