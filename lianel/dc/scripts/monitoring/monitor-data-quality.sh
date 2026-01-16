#!/bin/bash
# Data Quality Monitoring Script
# Checks data freshness, completeness, and quality metrics

set -euo pipefail

# Load environment variables
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

POSTGRES_HOST=${POSTGRES_HOST:-172.18.0.1}
POSTGRES_PORT=${POSTGRES_PORT:-5432}
POSTGRES_USER=${POSTGRES_USER:-postgres}
POSTGRES_DB=${POSTGRES_DB:-lianel_energy}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo "=== Data Quality Monitoring Report ==="
echo "Date: $(date)"
echo ""

# Check data freshness
echo "--- Data Freshness ---"
FRESHNESS_QUERY="SELECT EXTRACT(EPOCH FROM (NOW() - MAX(ingestion_date))) as age_seconds FROM meta_ingestion_log WHERE ingestion_date IS NOT NULL;"
AGE_SECONDS=$(PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d ${POSTGRES_DB} -t -c "${FRESHNESS_QUERY}" | xargs)

if [ -z "$AGE_SECONDS" ] || [ "$AGE_SECONDS" = "" ]; then
    echo -e "${RED}ERROR: No ingestion data found${NC}"
else
    AGE_HOURS=$(echo "scale=2; $AGE_SECONDS / 3600" | bc)
    if (( $(echo "$AGE_SECONDS > 86400" | bc -l) )); then
        echo -e "${RED}WARNING: Data is stale (${AGE_HOURS} hours old)${NC}"
    elif (( $(echo "$AGE_SECONDS > 43200" | bc -l) )); then
        echo -e "${YELLOW}WARNING: Data is getting old (${AGE_HOURS} hours old)${NC}"
    else
        echo -e "${GREEN}OK: Data is fresh (${AGE_HOURS} hours old)${NC}"
    fi
fi
echo ""

# Check data completeness
echo "--- Data Completeness ---"
COMPLETENESS_QUERY="SELECT CASE WHEN total > 0 THEN (non_null::float / total::float * 100) ELSE 0 END as completeness FROM (SELECT COUNT(*) as total, COUNT(*) FILTER (WHERE country_code IS NOT NULL AND year IS NOT NULL AND value IS NOT NULL) as non_null FROM fact_energy_annual) subq;"
COMPLETENESS=$(PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d ${POSTGRES_DB} -t -c "${COMPLETENESS_QUERY}" | xargs)

if [ -z "$COMPLETENESS" ] || [ "$COMPLETENESS" = "" ]; then
    echo -e "${RED}ERROR: Could not calculate completeness${NC}"
else
    if (( $(echo "$COMPLETENESS < 90" | bc -l) )); then
        echo -e "${RED}WARNING: Low completeness (${COMPLETENESS}%)${NC}"
    elif (( $(echo "$COMPLETENESS < 95" | bc -l) )); then
        echo -e "${YELLOW}WARNING: Completeness below target (${COMPLETENESS}%)${NC}"
    else
        echo -e "${GREEN}OK: Good completeness (${COMPLETENESS}%)${NC}"
    fi
fi
echo ""

# Check record counts
echo "--- Record Counts ---"
ENERGY_COUNT=$(PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d ${POSTGRES_DB} -t -c "SELECT COUNT(*) FROM fact_energy_annual;" | xargs)
GEO_COUNT=$(PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d ${POSTGRES_DB} -t -c "SELECT COUNT(*) FROM fact_geo_region_features;" | xargs)

echo "Energy records: ${ENERGY_COUNT:-0}"
echo "Geo features records: ${GEO_COUNT:-0}"

if [ "${ENERGY_COUNT:-0}" -eq 0 ]; then
    echo -e "${RED}WARNING: No energy data records${NC}"
fi
if [ "${GEO_COUNT:-0}" -eq 0 ]; then
    echo -e "${YELLOW}WARNING: No geo features data${NC}"
fi
echo ""

# Check ingestion status
echo "--- Recent Ingestion Status ---"
INGESTION_QUERY="SELECT source_name, COUNT(*) as count, MAX(ingestion_date) as last_ingestion FROM meta_ingestion_log WHERE ingestion_date > NOW() - INTERVAL '7 days' GROUP BY source_name ORDER BY last_ingestion DESC;"
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d ${POSTGRES_DB} -c "${INGESTION_QUERY}"
echo ""

echo "=== End of Report ==="
