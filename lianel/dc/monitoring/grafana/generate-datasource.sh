#!/bin/bash
# Generate Grafana datasource YAML with actual password from environment

set -e

PROVISIONING_DIR="/etc/grafana/provisioning/datasources"
TEMPLATE_FILE="${PROVISIONING_DIR}/datasources.yml.template"
OUTPUT_FILE="${PROVISIONING_DIR}/datasources.yml"

if [ -z "$POSTGRES_PASSWORD" ]; then
    echo "ERROR: POSTGRES_PASSWORD environment variable is not set"
    exit 1
fi

# If template exists, use it; otherwise create from scratch
if [ -f "$TEMPLATE_FILE" ]; then
    envsubst < "$TEMPLATE_FILE" > "$OUTPUT_FILE"
else
    # Generate datasource YAML directly
    cat > "$OUTPUT_FILE" <<EOF
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
EOF
fi

echo "✅ Datasource YAML generated successfully"
chmod 644 "$OUTPUT_FILE"
