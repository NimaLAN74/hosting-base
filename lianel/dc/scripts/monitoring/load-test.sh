#!/bin/bash
# Load Testing Script
# Tests API endpoints under various load conditions

set -e

echo "=========================================="
echo "Load Testing - Lianel Platform"
echo "Date: $(date)"
echo "=========================================="
echo ""

# Configuration
BASE_URL="${BASE_URL:-https://www.lianel.se}"
API_BASE="${API_BASE:-${BASE_URL}/api/v1}"
TOKEN="${TOKEN:-}"

# Check if Apache Bench (ab) is installed
if ! command -v ab &> /dev/null; then
    echo "Apache Bench not found. Installing..."
    sudo apt-get update
    sudo apt-get install -y apache2-utils
fi

# Check if wrk is installed (better alternative)
if ! command -v wrk &> /dev/null; then
    echo "wrk not found. Installing..."
    sudo apt-get install -y wrk
fi

echo "Load Test Configuration:"
echo "  Base URL: $BASE_URL"
echo "  API Base: $API_BASE"
echo "  Token: ${TOKEN:0:20}..." # Show first 20 chars only
echo ""

# Test scenarios
echo "1. Health Check Endpoint (No Auth)"
echo "-----------------------------------"
echo "Testing: GET ${API_BASE}/health"
wrk -t4 -c100 -d30s --timeout 10s "${API_BASE}/health" || ab -n 10000 -c 100 "${API_BASE}/health"
echo ""

if [ -n "$TOKEN" ]; then
    echo "2. API Endpoint with Authentication"
    echo "-----------------------------------"
    echo "Testing: GET ${API_BASE}/datasets/forecasting?limit=10"
    
    # Create temporary file with headers
    HEADERS_FILE=$(mktemp)
    echo "Authorization: Bearer $TOKEN" > "$HEADERS_FILE"
    
    wrk -t4 -c50 -d30s --timeout 10s \
        -H "Authorization: Bearer $TOKEN" \
        "${API_BASE}/datasets/forecasting?limit=10" || \
    ab -n 5000 -c 50 -H "Authorization: Bearer $TOKEN" \
        "${API_BASE}/datasets/forecasting?limit=10"
    
    rm -f "$HEADERS_FILE"
    echo ""
    
    echo "3. Concurrent Users Test"
    echo "-----------------------------------"
    echo "Testing with 100 concurrent users for 60 seconds"
    wrk -t8 -c100 -d60s --timeout 10s \
        -H "Authorization: Bearer $TOKEN" \
        "${API_BASE}/datasets/forecasting?limit=10" || \
    ab -n 10000 -c 100 -H "Authorization: Bearer $TOKEN" \
        "${API_BASE}/datasets/forecasting?limit=10"
    echo ""
    
    echo "4. Stress Test"
    echo "-----------------------------------"
    echo "Testing with 200 concurrent users for 30 seconds"
    wrk -t8 -c200 -d30s --timeout 10s \
        -H "Authorization: Bearer $TOKEN" \
        "${API_BASE}/datasets/forecasting?limit=10" || \
    ab -n 20000 -c 200 -H "Authorization: Bearer $TOKEN" \
        "${API_BASE}/datasets/forecasting?limit=10"
    echo ""
else
    echo "2. Skipping authenticated tests (no token provided)"
    echo "   Set TOKEN environment variable to test authenticated endpoints"
    echo ""
fi

echo "=========================================="
echo "Load Testing Complete"
echo "=========================================="
echo ""
echo "Review the results above for:"
echo "  - Requests per second (RPS)"
echo "  - Latency (p50, p95, p99)"
echo "  - Error rate"
echo "  - Throughput"
echo ""
