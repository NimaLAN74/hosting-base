#!/bin/bash
# Update Grafana PostgreSQL datasource password
# This script reads POSTGRES_PASSWORD from .env and updates the Grafana datasource YAML

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DC_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
ENV_FILE="${DC_DIR}/.env"
GRAFANA_CONTAINER="grafana"
DATASOURCE_FILE="/etc/grafana/provisioning/datasources/datasources.yml"

# Read password from .env file
if [ -f "$ENV_FILE" ]; then
    POSTGRES_PASSWORD=$(grep "^POSTGRES_PASSWORD=" "$ENV_FILE" | cut -d'=' -f2- | tr -d '"' | tr -d "'")
else
    echo "ERROR: .env file not found at $ENV_FILE"
    exit 1
fi

if [ -z "$POSTGRES_PASSWORD" ]; then
    echo "ERROR: POSTGRES_PASSWORD not found in .env file"
    exit 1
fi

echo "Updating Grafana PostgreSQL datasource password..."

# Generate datasource YAML with actual password
docker exec "$GRAFANA_CONTAINER" bash -c "cat > $DATASOURCE_FILE <<'ENDOFFILE'
apiVersion: 1

datasources:
  - name: Loki
    type: loki
    access: proxy
    url: http://loki:3100
    isDefault: false
    editable: true

  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: false
    editable: true
    uid: prometheus

  - name: PostgreSQL Energy
    type: postgres
    access: proxy
    url: 172.18.0.1:5432
    database: lianel_energy
    user: airflow
    uid: postgres-energy
    secureJsonData:
      password: ${POSTGRES_PASSWORD}
    jsonData:
      sslmode: disable
      maxOpenConns: 100
      maxIdleConns: 100
      connMaxLifetime: 14400
      postgresVersion: 1500
      timescaledb: false
    isDefault: false
    editable: true
ENDOFFILE
"

# Replace the password placeholder with actual password
docker exec "$GRAFANA_CONTAINER" sed -i "s|\${POSTGRES_PASSWORD}|${POSTGRES_PASSWORD}|g" "$DATASOURCE_FILE"

echo "✅ Datasource password updated"
echo "Restarting Grafana to apply changes..."

docker restart "$GRAFANA_CONTAINER"

echo "✅ Grafana restarted. Datasource should now work correctly."
