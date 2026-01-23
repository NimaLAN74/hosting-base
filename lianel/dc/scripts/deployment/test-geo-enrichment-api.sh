#!/bin/bash
# Script to test the geo-enrichment API endpoint with coordinates
# Usage: Run this script to verify API returns data with latitude/longitude

set -e

echo "=== Testing Geo-Enrichment API with Coordinates ==="
echo ""

API_BASE_URL="https://www.lianel.se/api/v1/datasets/geo-enrichment"
KEYCLOAK_URL="${KEYCLOAK_URL:-https://auth.lianel.se}"
KEYCLOAK_REALM="${KEYCLOAK_REALM:-lianel}"
KEYCLOAK_CLIENT_ID="${KEYCLOAK_CLIENT_ID:-frontend-client}"
KEYCLOAK_ADMIN_USER="${KEYCLOAK_ADMIN_USER}"
KEYCLOAK_ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD}"

# Try to load environment variables from .env file
if [ -f ".env" ]; then
    echo "Loaded environment from: .env"
    source .env
fi

if [ -z "$KEYCLOAK_ADMIN_USER" ] || [ -z "$KEYCLOAK_ADMIN_PASSWORD" ]; then
    echo "ERROR: KEYCLOAK_ADMIN_USER or KEYCLOAK_ADMIN_PASSWORD not set in environment or .env file."
    echo "Please ensure these are set for API authentication."
    exit 1
fi

# 1. Get JWT token
echo "1. Getting JWT token from Keycloak..."
TOKEN_RESPONSE=$(curl -s -X POST "$KEYCLOAK_URL/realms/$KEYCLOAK_REALM/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "client_id=$KEYCLOAK_CLIENT_ID" \
  -d "username=$KEYCLOAK_ADMIN_USER" \
  -d "password=$KEYCLOAK_ADMIN_PASSWORD" \
  -d "grant_type=password")

ACCESS_TOKEN=$(echo "$TOKEN_RESPONSE" | python3 -c "import sys, json; print(json.load(sys.stdin).get('access_token', ''))" 2>/dev/null || echo "")

if [ -z "$ACCESS_TOKEN" ] || [ "$ACCESS_TOKEN" == "null" ]; then
    echo "❌ Failed to get access token."
    echo "Response: $TOKEN_RESPONSE"
    exit 1
fi
echo "   ✅ Access token obtained."
echo ""

# Function to test API endpoint
test_api_endpoint() {
    local url="$1"
    local description="$2"
    echo "$description"
    RESPONSE=$(curl -s -w "\n%{http_code}" -H "Authorization: Bearer $ACCESS_TOKEN" "$url")
    HTTP_CODE=$(echo "$RESPONSE" | tail -1)
    BODY=$(echo "$RESPONSE" | sed '$d')

    echo "   Status: $HTTP_CODE"
    if [ "$HTTP_CODE" -ne 200 ]; then
        echo "   ❌ Error Response: $BODY"
        echo ""
        return 1
    fi
    
    # Check if response has coordinates
    HAS_COORDS=$(echo "$BODY" | python3 -c "import sys, json; data=json.load(sys.stdin); print('yes' if data.get('data') and len(data['data']) > 0 and data['data'][0].get('latitude') and data['data'][0].get('longitude') else 'no')" 2>/dev/null || echo "no")
    
    if [ "$HAS_COORDS" == "yes" ]; then
        echo "   ✅ Response contains coordinates"
        echo "   Sample record:"
        echo "$BODY" | python3 -c "import sys, json; data=json.load(sys.stdin); rec=data['data'][0] if data.get('data') else {}; print(f\"     Region: {rec.get('region_id', 'N/A')}\"); print(f\"     Country: {rec.get('cntr_code', 'N/A')}\"); print(f\"     Year: {rec.get('year', 'N/A')}\"); print(f\"     Latitude: {rec.get('latitude', 'N/A')}\"); print(f\"     Longitude: {rec.get('longitude', 'N/A')}\"); print(f\"     OSM Features: {rec.get('osm_feature_count', 'N/A')}\"); print(f\"     Power Plants: {rec.get('power_plant_count', 'N/A')}\")" 2>/dev/null || echo "     (Could not parse response)"
    else
        echo "   ⚠️  Response does NOT contain coordinates"
        echo "   Response preview:"
        echo "$BODY" | python3 -m json.tool 2>/dev/null | head -20 || echo "$BODY" | head -20
    fi
    echo ""
    return 0
}

# 2. Testing basic endpoint (no filters)
test_api_endpoint "$API_BASE_URL?limit=5" "2. Testing basic endpoint (limit=5)..."

# 3. Testing with country filter (SE)
test_api_endpoint "$API_BASE_URL?cntr_code=SE&limit=5" "3. Testing with country filter (SE, limit=5)..."

# 4. Testing with year filter (2024)
test_api_endpoint "$API_BASE_URL?year=2024&limit=5" "4. Testing with year filter (2024, limit=5)..."

# 5. Summary
echo "=== API Testing Complete ==="
echo ""
echo "Summary:"
echo "  - API endpoint: $API_BASE_URL"
echo "  - Authentication: ✅ Working"
echo "  - Coordinates: $([ \"$HAS_COORDS\" == \"yes\" ] && echo \"✅ Present\" || echo \"❌ Missing\")"
echo ""
