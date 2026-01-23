#!/bin/bash
# Create fact_electricity_timeseries table if it doesn't exist
# This script should be run on the remote host

set -euo pipefail

REPO_DIR="${REPO_DIR:-/root/hosting-base/lianel/dc}"

echo "=== Creating fact_electricity_timeseries Table ==="
echo "Repository: $REPO_DIR"
echo ""

cd "$REPO_DIR" || exit 1

# Find PostgreSQL container
POSTGRES_CONTAINER=$(docker ps --format '{{.Names}}' | grep -i postgres | head -1)

if [ -z "$POSTGRES_CONTAINER" ]; then
  echo "Error: PostgreSQL container not found"
  exit 1
fi

echo "Found PostgreSQL container: $POSTGRES_CONTAINER"
echo ""

# Source .env to get database credentials
if [ -f ".env" ]; then
  source .env
fi

# Run migration
if [ -f "database/migrations/003_create_fact_electricity_timeseries.sql" ]; then
  echo "Running migration..."
  docker exec -i "$POSTGRES_CONTAINER" psql -U "${POSTGRES_USER:-airflow}" -d lianel_energy < database/migrations/003_create_fact_electricity_timeseries.sql
  echo "✅ Migration completed"
else
  echo "Error: Migration file not found at database/migrations/003_create_fact_electricity_timeseries.sql"
  exit 1
fi

# Verify table exists
echo ""
echo "Verifying table creation..."
docker exec -i "$POSTGRES_CONTAINER" psql -U "${POSTGRES_USER:-airflow}" -d lianel_energy -c "\d fact_electricity_timeseries" || echo "Warning: Could not describe table"

echo ""
echo "=== Table Creation Complete ==="
echo "You can now run the ENTSO-E ingestion DAG to populate the table."
