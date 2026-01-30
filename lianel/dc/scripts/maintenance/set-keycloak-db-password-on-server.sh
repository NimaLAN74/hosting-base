#!/bin/bash
# Set Postgres password for user keycloak to match KEYCLOAK_DB_PASSWORD from .env.
# Run ON THE SERVER. Use when Keycloak fails with "password authentication failed for user keycloak".
set -e

DC_DIR=""
for d in /root/hosting-base/lianel/dc /root/lianel/dc .; do
  [ -f "$d/.env" ] && DC_DIR="$d" && break
done
[ -z "$DC_DIR" ] && { echo "ERROR: .env not found"; exit 1; }
cd "$DC_DIR" && source .env

# Escape single quotes for SQL: ' -> ''
PASS_ESC="${KEYCLOAK_DB_PASSWORD//\'/\'\'}"
export PGPASSWORD="${POSTGRES_PASSWORD}"
psql -h "${POSTGRES_HOST:-172.18.0.1}" -p "${POSTGRES_PORT:-5432}" -U postgres -d postgres -c "ALTER USER keycloak WITH PASSWORD '${PASS_ESC}';"
echo "Keycloak DB user password updated. Restart Keycloak: docker restart keycloak"
