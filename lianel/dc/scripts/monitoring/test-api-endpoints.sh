#!/bin/bash
# Test API Endpoints for Phase 5 Validation
# Usage: ./test-api-endpoints.sh [BASE_URL]

set -e

BASE_URL="${1:-https://www.lianel.se}"
API_BASE="${BASE_URL}/api/v1"

echo "=== Testing API Endpoints ==="
echo "Base URL: ${BASE_URL}"
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

test_endpoint() {
    local name="$1"
    local url="$2"
    local expected_status="${3:-200}"
    
    echo -n "Testing ${name}... "
    
    response=$(curl -s -w "\n%{http_code}" -k "${url}" 2>&1)
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | sed '$d')
    
    if [ "$http_code" = "$expected_status" ]; then
        echo -e "${GREEN}✓${NC} (HTTP ${http_code})"
        
        # Check if response is valid JSON
        if echo "$body" | jq . >/dev/null 2>&1; then
            total=$(echo "$body" | jq -r '.total // .data | length // 0' 2>/dev/null)
            if [ -n "$total" ] && [ "$total" != "null" ]; then
                echo "  → Found ${total} records"
            fi
        fi
        return 0
    else
        echo -e "${RED}✗${NC} (HTTP ${http_code}, expected ${expected_status})"
        echo "  Response: ${body:0:200}"
        return 1
    fi
}

# Test ML Dataset Endpoints
echo "## ML Dataset Endpoints"
test_endpoint "Forecasting Dataset" "${API_BASE}/datasets/forecasting?limit=5"
test_endpoint "Clustering Dataset" "${API_BASE}/datasets/clustering?limit=5"
test_endpoint "Geo Enrichment Dataset" "${API_BASE}/datasets/geo-enrichment?limit=5"

echo ""

# Test ENTSO-E Endpoints
echo "## ENTSO-E Electricity Endpoints"
test_endpoint "Electricity Timeseries" "${API_BASE}/electricity/timeseries?limit=5"
test_endpoint "Electricity Timeseries (filtered)" "${API_BASE}/electricity/timeseries?country_code=SE&limit=5"

echo ""

# Test OSM Endpoints
echo "## OSM Geo Features Endpoints"
test_endpoint "Geo Features" "${API_BASE}/geo/features?limit=5"
test_endpoint "Geo Features (filtered)" "${API_BASE}/geo/features?region_id=SE11&limit=5"

echo ""

# Test Swagger UI
echo "## Documentation"
test_endpoint "Swagger UI" "${BASE_URL}/swagger-ui/" "200"

echo ""
echo "=== Test Summary ==="
echo "Note: Some endpoints may require authentication (401/403 is expected)"
echo "For authenticated testing, use browser or provide Bearer token"