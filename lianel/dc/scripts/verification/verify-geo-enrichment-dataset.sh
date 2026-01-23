#!/bin/bash
# Script to verify geo-enrichment dataset has OSM features
# Usage: Run this script to check dataset completeness and OSM feature inclusion

set -e

echo "=== Geo-Enrichment Dataset Verification ==="
echo ""

# Database connection details (from environment or defaults)
DB_HOST="${POSTGRES_HOST:-172.18.0.1}"
DB_PORT="${POSTGRES_PORT:-5432}"
DB_USER="${POSTGRES_USER:-postgres}"
DB_NAME="${POSTGRES_DB:-lianel_energy}"
PGPASSWORD="${POSTGRES_PASSWORD}"

if [ -z "$PGPASSWORD" ]; then
    echo "ERROR: POSTGRES_PASSWORD not set"
    exit 1
fi

export PGPASSWORD

echo "1. Checking total records in ml_dataset_geo_enrichment_v1..."
TOTAL_RECORDS=$(psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT COUNT(*) FROM ml_dataset_geo_enrichment_v1;" | tr -d ' ')
echo "   Total records: $TOTAL_RECORDS"
echo ""

echo "2. Checking records with OSM features..."
OSM_RECORDS=$(psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT COUNT(*) FROM ml_dataset_geo_enrichment_v1 WHERE osm_feature_count > 0;" | tr -d ' ')
echo "   Records with OSM features: $OSM_RECORDS"
if [ "$TOTAL_RECORDS" -gt 0 ]; then
    OSM_PERCENTAGE=$(echo "scale=2; $OSM_RECORDS * 100 / $TOTAL_RECORDS" | bc)
    echo "   OSM coverage: ${OSM_PERCENTAGE}%"
fi
echo ""

echo "3. Checking OSM feature columns..."
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
SELECT 
    COUNT(*) as total_records,
    COUNT(osm_feature_count) as records_with_feature_count,
    COUNT(osm_power_plants) as records_with_power_plants,
    COUNT(osm_industrial_areas) as records_with_industrial,
    COUNT(osm_buildings) as records_with_buildings,
    COUNT(osm_transport) as records_with_transport,
    SUM(osm_feature_count) as total_osm_features,
    SUM(osm_power_plants) as total_power_plants,
    SUM(osm_industrial_areas) as total_industrial,
    SUM(osm_buildings) as total_buildings,
    SUM(osm_transport) as total_transport
FROM ml_dataset_geo_enrichment_v1;
"
echo ""

echo "4. Sample records with OSM features (top 10)..."
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
SELECT 
    region_id,
    country_code,
    year,
    osm_feature_count,
    osm_power_plants,
    osm_industrial_areas,
    osm_buildings,
    osm_transport,
    total_final_energy_gwh
FROM ml_dataset_geo_enrichment_v1
WHERE osm_feature_count > 0
ORDER BY osm_feature_count DESC
LIMIT 10;
"
echo ""

echo "5. Checking data by country..."
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
SELECT 
    country_code,
    COUNT(*) as total_records,
    COUNT(CASE WHEN osm_feature_count > 0 THEN 1 END) as records_with_osm,
    SUM(osm_feature_count) as total_osm_features,
    AVG(osm_feature_count) as avg_osm_features
FROM ml_dataset_geo_enrichment_v1
GROUP BY country_code
ORDER BY country_code;
"
echo ""

echo "6. Checking data by year..."
psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
SELECT 
    year,
    COUNT(*) as total_records,
    COUNT(CASE WHEN osm_feature_count > 0 THEN 1 END) as records_with_osm,
    SUM(osm_feature_count) as total_osm_features
FROM ml_dataset_geo_enrichment_v1
GROUP BY year
ORDER BY year;
"
echo ""

echo "7. Verifying fact_geo_region_features table..."
OSM_TABLE_COUNT=$(psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT COUNT(*) FROM fact_geo_region_features;" | tr -d ' ')
echo "   Records in fact_geo_region_features: $OSM_TABLE_COUNT"
echo ""

echo "=== Verification Complete ==="
echo ""
echo "Summary:"
echo "  - Total records in dataset: $TOTAL_RECORDS"
echo "  - Records with OSM features: $OSM_RECORDS"
echo "  - OSM source table records: $OSM_TABLE_COUNT"
echo ""

if [ "$OSM_RECORDS" -gt 0 ]; then
    echo "✅ SUCCESS: Geo-enrichment dataset contains OSM features"
    exit 0
else
    echo "⚠️  WARNING: No OSM features found in dataset"
    exit 1
fi
