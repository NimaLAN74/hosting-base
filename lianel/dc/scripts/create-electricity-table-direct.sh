#!/bin/bash
# Create fact_electricity_timeseries table directly via psql
# This assumes PostgreSQL is accessible on the host at 172.18.0.1:5432 or localhost:5432

set -euo pipefail

REPO_DIR="${REPO_DIR:-/root/hosting-base/lianel/dc}"

echo "=== Creating fact_electricity_timeseries Table ==="
echo "Repository: $REPO_DIR"
echo ""

cd "$REPO_DIR" || exit 1

# Source .env to get database credentials
if [ -f ".env" ]; then
  source .env
fi

DB_USER="${POSTGRES_USER:-airflow}"
DB_NAME="lianel_energy"
DB_HOST="${POSTGRES_HOST:-172.18.0.1}"
DB_PORT="${POSTGRES_PORT:-5432}"

echo "Database: $DB_NAME"
echo "User: $DB_USER"
echo "Host: $DB_HOST:$DB_PORT"
echo ""

# Check if we can use psql directly or need to use docker exec
if command -v psql >/dev/null 2>&1; then
  echo "Using psql from host..."
  if [ -z "${POSTGRES_PASSWORD:-}" ]; then
    echo "Error: POSTGRES_PASSWORD not set in .env file"
    echo "Please set POSTGRES_PASSWORD in .env or export it before running this script"
    exit 1
  fi
  export PGPASSWORD="${POSTGRES_PASSWORD}"
  psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -f database/migrations/003_create_fact_electricity_timeseries.sql
elif docker ps --format '{{.Names}}' | grep -q 'lianel-energy-service'; then
  echo "Using docker exec to run via energy service..."
  # The energy service has database access, we can use it to run SQL
  docker exec -i lianel-energy-service sh -c "PGPASSWORD=\${POSTGRES_PASSWORD} psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME" < database/migrations/003_create_fact_electricity_timeseries.sql
else
  echo "Error: Cannot find psql or energy service container"
  echo "Please run the migration manually:"
  echo "  psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -f database/migrations/003_create_fact_electricity_timeseries.sql"
  exit 1
fi

echo ""
echo "✅ Migration completed"

# Verify table exists
echo ""
echo "Verifying table creation..."
if command -v psql >/dev/null 2>&1; then
  export PGPASSWORD="${POSTGRES_PASSWORD}"
  psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "\d fact_electricity_timeseries" || echo "Warning: Could not describe table"
else
  docker exec -i lianel-energy-service sh -c "PGPASSWORD=\${POSTGRES_PASSWORD} psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d $DB_NAME -c '\d fact_electricity_timeseries'" || echo "Warning: Could not describe table"
fi

echo ""
echo "=== Table Creation Complete ==="
echo "You can now run the ENTSO-E ingestion DAG to populate the table."
