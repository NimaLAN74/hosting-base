#!/bin/bash
# Grafana datasource provisioning script
# Substitutes environment variables in datasources.yml

PROVISIONING_DIR="/etc/grafana/provisioning"
DATASOURCES_FILE="${PROVISIONING_DIR}/datasources/datasources.yml"

# If POSTGRES_PASSWORD is not set, try to get it from environment
if [ -z "$POSTGRES_PASSWORD" ]; then
    echo "Warning: POSTGRES_PASSWORD not set in environment"
    # Try to read from .env file if it exists
    if [ -f "/etc/grafana/.env" ]; then
        export $(grep POSTGRES_PASSWORD /etc/grafana/.env | xargs)
    fi
fi

# If we have the password, substitute it in the datasources file
if [ -n "$POSTGRES_PASSWORD" ]; then
    # Create a backup
    cp "$DATASOURCES_FILE" "${DATASOURCES_FILE}.bak" 2>/dev/null || true
    
    # Substitute the password using sed
    sed -i "s|\${POSTGRES_PASSWORD}|${POSTGRES_PASSWORD}|g" "$DATASOURCES_FILE"
    
    echo "Datasource passwords substituted successfully"
else
    echo "Error: POSTGRES_PASSWORD not available for substitution"
    exit 1
fi
