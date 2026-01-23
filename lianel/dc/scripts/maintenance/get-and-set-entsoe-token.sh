#!/bin/bash
# Script to set ENTSO-E API token in Airflow Variables
# Usage: ./get-and-set-entsoe-token.sh <token>
# Note: ENTSO-E tokens require manual registration at https://transparency.entsoe.eu/

set -e

TOKEN="${1:-}"

if [ -z "$TOKEN" ]; then
    echo "❌ Error: ENTSO-E API token is required"
    echo "Usage: $0 <token>"
    echo ""
    echo "To get a token:"
    echo "1. Register at https://transparency.entsoe.eu/"
    echo "2. Request API access (email: transparency@entsoe.eu)"
    echo "3. Get your security token from the dashboard"
    exit 1
fi

echo "=== Setting ENTSO-E API Token ==="
echo ""

# Set the token in Airflow Variables
docker exec dc-airflow-apiserver-1 airflow variables set ENTSOE_API_TOKEN "$TOKEN" 2>&1 | grep -v 'RemovedInAirflow4Warning' | grep -v 'deprecated' || {
    echo "❌ Failed to set token in Airflow"
    exit 1
}

echo "✅ Token set successfully in Airflow Variables"
echo ""

# Verify token is set
echo "Verifying token..."
VERIFIED=$(docker exec dc-airflow-apiserver-1 airflow variables get ENTSOE_API_TOKEN 2>&1 | grep -v 'RemovedInAirflow4Warning' | grep -v 'deprecated' | tail -1)

if [ -n "$VERIFIED" ] && [ "$VERIFIED" != "Variable ENTSOE_API_TOKEN does not exist" ]; then
    TOKEN_PREVIEW="${VERIFIED:0:15}..."
    echo "✅ Token verified: $TOKEN_PREVIEW"
    echo ""
    
    # Test API call with token
    echo "Testing ENTSO-E API with token..."
    TEST_RESPONSE=$(curl -s "https://web-api.tp.entsoe.eu/api?securityToken=$TOKEN&documentType=A65&processType=A16&outBiddingZone_Domain=10YSE-1--------C&periodStart=202501180000&periodEnd=202501190000" 2>&1)
    
    if echo "$TEST_RESPONSE" | grep -q "<?xml" && ! echo "$TEST_RESPONSE" | grep -qi "Authentication failed\|999"; then
        echo "✅ API test successful - received valid XML response"
        echo "   Response preview: $(echo "$TEST_RESPONSE" | head -3 | tr '\n' ' ')"
    elif echo "$TEST_RESPONSE" | grep -qi "Authentication failed\|999\|401\|403"; then
        echo "⚠️  API test returned authentication error"
        echo "   This may mean the token is invalid or expired"
        echo "   Response: $(echo "$TEST_RESPONSE" | grep -o '<text>.*</text>' | head -1)"
    else
        echo "⚠️  Could not verify API response"
        echo "   Response preview: $(echo "$TEST_RESPONSE" | head -3)"
    fi
    
    echo ""
    echo "Next steps:"
    echo "1. Trigger the entsoe_ingestion DAG in Airflow"
    echo "2. Monitor the DAG run to verify data is being ingested"
    echo "3. Check the API endpoint: https://www.lianel.se/api/v1/electricity/timeseries?limit=1"
else
    echo "⚠️  Warning: Could not verify token"
fi

echo ""
echo "=========================================="
