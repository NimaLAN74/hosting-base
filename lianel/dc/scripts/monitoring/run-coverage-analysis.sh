#!/bin/bash
# Run data coverage analysis script
# This script can be run on the remote host to analyze data coverage

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

# Load environment variables if .env exists
if [ -f .env ]; then
    export $(grep -v '^#' .env | xargs)
fi

# Set database connection from docker-compose or environment
export POSTGRES_HOST="${POSTGRES_HOST:-localhost}"
export POSTGRES_PORT="${POSTGRES_PORT:-5432}"
export POSTGRES_DB="${POSTGRES_DB:-lianel_energy}"
export POSTGRES_USER="${POSTGRES_USER:-airflow}"
export POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-airflow}"

# Check if running in Docker or locally
if [ -f /.dockerenv ] || [ -n "${DOCKER_CONTAINER:-}" ]; then
    # Running in Docker - use Python from container
    python3 scripts/analyze-data-coverage.py
else
    # Running locally - check if we need to install dependencies
    if ! python3 -c "import psycopg2" 2>/dev/null; then
        echo "Installing psycopg2..."
        pip3 install psycopg2-binary
    fi
    python3 scripts/analyze-data-coverage.py
fi
