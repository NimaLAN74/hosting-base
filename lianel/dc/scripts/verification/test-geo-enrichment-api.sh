#!/bin/bash
# Script to test geo-enrichment API endpoints
# Usage: Run this script to verify API endpoints return OSM data correctly

set -e

echo "=== Geo-Enrichment API Testing ==="
echo ""

# API base URL (from environment or default)
API_BASE="${API_BASE_URL:-https://www.lianel.se}"
ENDPOINT="${API_BASE}/api/v1/datasets/geo-enrichment"

echo "Testing endpoint: $ENDPOINT"
echo ""

echo "1. Testing basic endpoint (no filters)..."
RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" "$ENDPOINT?limit=5")
HTTP_CODE=$(echo "$RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | sed '/HTTP_CODE/d')

if [ "$HTTP_CODE" = "200" ]; then
    echo "   ✅ Status: 200 OK"
    echo "   Response preview:"
    echo "$BODY" | head -20
else
    echo "   ❌ Status: $HTTP_CODE"
    echo "   Response: $BODY"
fi
echo ""

echo "2. Testing with country filter (SE)..."
RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" "$ENDPOINT?country_code=SE&limit=5")
HTTP_CODE=$(echo "$RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | sed '/HTTP_CODE/d')

if [ "$HTTP_CODE" = "200" ]; then
    echo "   ✅ Status: 200 OK"
    echo "   Response preview:"
    echo "$BODY" | head -20
else
    echo "   ❌ Status: $HTTP_CODE"
    echo "   Response: $BODY"
fi
echo ""

echo "3. Testing with year filter (2020)..."
RESPONSE=$(curl -s -w "\nHTTP_CODE:%{http_code}" "$ENDPOINT?year=2020&limit=5")
HTTP_CODE=$(echo "$RESPONSE" | grep "HTTP_CODE" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | sed '/HTTP_CODE/d')

if [ "$HTTP_CODE" = "200" ]; then
    echo "   ✅ Status: 200 OK"
    echo "   Response preview:"
    echo "$BODY" | head -20
else
    echo "   ❌ Status: $HTTP_CODE"
    echo "   Response: $BODY"
fi
echo ""

echo "4. Checking if response contains OSM fields..."
RESPONSE=$(curl -s "$ENDPOINT?limit=1")
if echo "$RESPONSE" | grep -q "osm_feature_count\|osm_power_plants\|osm_industrial_areas"; then
    echo "   ✅ OSM fields found in response"
    echo "   Sample OSM fields:"
    echo "$RESPONSE" | grep -o '"osm_[^"]*":[^,}]*' | head -5
else
    echo "   ⚠️  No OSM fields found in response"
    echo "   Response preview:"
    echo "$RESPONSE" | head -10
fi
echo ""

echo "=== API Testing Complete ==="
