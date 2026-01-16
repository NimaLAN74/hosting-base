#!/bin/bash
# Pre-start script to substitute POSTGRES_PASSWORD in datasources.yml on the host

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATASOURCES_FILE="${SCRIPT_DIR}/provisioning/datasources/datasources.yml"
ENV_FILE="${SCRIPT_DIR}/../../.env"

# Source .env file if it exists
if [ -f "$ENV_FILE" ]; then
    export $(grep -v '^#' "$ENV_FILE" | grep POSTGRES_PASSWORD | xargs)
fi

# Check if password is set
if [ -z "$POSTGRES_PASSWORD" ]; then
    echo "Warning: POSTGRES_PASSWORD not found in environment"
    exit 0  # Don't fail, just skip substitution
fi

# Create backup
if [ -f "$DATASOURCES_FILE" ]; then
    cp "$DATASOURCES_FILE" "${DATASOURCES_FILE}.bak" 2>/dev/null || true
    
    # Substitute password (escape special chars in password)
    PASSWORD_ESCAPED=$(echo "$POSTGRES_PASSWORD" | sed 's/[[\.*^$()+?{|]/\\&/g')
    sed -i.tmp "s|\${POSTGRES_PASSWORD}|${PASSWORD_ESCAPED}|g" "$DATASOURCES_FILE" 2>/dev/null || true
    rm -f "${DATASOURCES_FILE}.tmp" 2>/dev/null || true
    
    echo "✅ Substituted POSTGRES_PASSWORD in datasources.yml"
fi
