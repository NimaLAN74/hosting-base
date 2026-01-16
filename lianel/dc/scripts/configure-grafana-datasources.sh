#!/bin/bash
# Configure Grafana datasources via API
# This ensures passwords are set correctly

set -e

GRAFANA_URL="${GRAFANA_URL:-http://localhost:3000}"
GRAFANA_USER="${GRAFANA_ADMIN_USER:-admin}"
GRAFANA_PASSWORD="${GRAFANA_ADMIN_PASSWORD:-admin}"
POSTGRES_PASSWORD="${POSTGRES_PASSWORD}"

if [ -z "$POSTGRES_PASSWORD" ]; then
    echo "Error: POSTGRES_PASSWORD not set"
    exit 1
fi

echo "Configuring Grafana datasources..."

# Wait for Grafana to be ready
for i in {1..30}; do
    if curl -s -f -u "${GRAFANA_USER}:${GRAFANA_PASSWORD}" "${GRAFANA_URL}/api/health" > /dev/null 2>&1; then
        break
    fi
    echo "Waiting for Grafana... ($i/30)"
    sleep 2
done

# Configure PostgreSQL Airflow datasource
curl -X PUT \
  -u "${GRAFANA_USER}:${GRAFANA_PASSWORD}" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"PostgreSQL Airflow\",
    \"type\": \"postgres\",
    \"access\": \"proxy\",
    \"url\": \"172.18.0.1:5432\",
    \"database\": \"airflow\",
    \"user\": \"airflow\",
    \"uid\": \"postgres-airflow\",
    \"secureJsonData\": {
      \"password\": \"${POSTGRES_PASSWORD}\"
    },
    \"jsonData\": {
      \"sslmode\": \"disable\",
      \"maxOpenConns\": 100,
      \"maxIdleConns\": 100,
      \"connMaxLifetime\": 14400,
      \"postgresVersion\": 1500,
      \"timescaledb\": false,
      \"search_path\": \"public\"
    },
    \"isDefault\": false,
    \"editable\": true
  }" \
  "${GRAFANA_URL}/api/datasources/uid/postgres-airflow" 2>/dev/null || \
curl -X POST \
  -u "${GRAFANA_USER}:${GRAFANA_PASSWORD}" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"PostgreSQL Airflow\",
    \"type\": \"postgres\",
    \"access\": \"proxy\",
    \"url\": \"172.18.0.1:5432\",
    \"database\": \"airflow\",
    \"user\": \"airflow\",
    \"uid\": \"postgres-airflow\",
    \"secureJsonData\": {
      \"password\": \"${POSTGRES_PASSWORD}\"
    },
    \"jsonData\": {
      \"sslmode\": \"disable\",
      \"maxOpenConns\": 100,
      \"maxIdleConns\": 100,
      \"connMaxLifetime\": 14400,
      \"postgresVersion\": 1500,
      \"timescaledb\": false,
      \"search_path\": \"public\"
    },
    \"isDefault\": false,
    \"editable\": true
  }" \
  "${GRAFANA_URL}/api/datasources" 2>/dev/null

echo "✅ Configured PostgreSQL Airflow datasource"

# Configure PostgreSQL Energy datasource
curl -X PUT \
  -u "${GRAFANA_USER}:${GRAFANA_PASSWORD}" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"PostgreSQL Energy\",
    \"type\": \"postgres\",
    \"access\": \"proxy\",
    \"url\": \"172.18.0.1:5432\",
    \"database\": \"lianel_energy\",
    \"user\": \"airflow\",
    \"uid\": \"postgres-energy\",
    \"secureJsonData\": {
      \"password\": \"${POSTGRES_PASSWORD}\"
    },
    \"jsonData\": {
      \"sslmode\": \"disable\",
      \"maxOpenConns\": 100,
      \"maxIdleConns\": 100,
      \"connMaxLifetime\": 14400,
      \"postgresVersion\": 1500,
      \"timescaledb\": false
    },
    \"isDefault\": false,
    \"editable\": true
  }" \
  "${GRAFANA_URL}/api/datasources/uid/postgres-energy" 2>/dev/null || \
curl -X POST \
  -u "${GRAFANA_USER}:${GRAFANA_PASSWORD}" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"PostgreSQL Energy\",
    \"type\": \"postgres\",
    \"access\": \"proxy\",
    \"url\": \"172.18.0.1:5432\",
    \"database\": \"lianel_energy\",
    \"user\": \"airflow\",
    \"uid\": \"postgres-energy\",
    \"secureJsonData\": {
      \"password\": \"${POSTGRES_PASSWORD}\"
    },
    \"jsonData\": {
      \"sslmode\": \"disable\",
      \"maxOpenConns\": 100,
      \"maxIdleConns\": 100,
      \"connMaxLifetime\": 14400,
      \"postgresVersion\": 1500,
      \"timescaledb\": false
    },
    \"isDefault\": false,
    \"editable\": true
  }" \
  "${GRAFANA_URL}/api/datasources" 2>/dev/null

echo "✅ Configured PostgreSQL Energy datasource"
echo "✅ All datasources configured successfully"
