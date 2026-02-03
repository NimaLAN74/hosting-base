#!/bin/bash
# Set Postgres password for user keycloak to match KEYCLOAK_DB_PASSWORD from .env.
# Run ON THE SERVER. Use when Keycloak fails with "password authentication failed for user keycloak".
# Supports: psql on PATH, or psql via docker exec (when Postgres runs in Docker).
set -e

DC_DIR=""
for d in /root/hosting-base/lianel/dc /root/lianel/dc /root/hosting-base/lianel/dc .; do
  [ -f "$d/.env" ] && DC_DIR="$d" && break
done
[ -z "$DC_DIR" ] && { echo "ERROR: .env not found in /root/hosting-base/lianel/dc, /root/lianel/dc, or ."; exit 1; }
cd "$DC_DIR" && source .env

[ -z "${KEYCLOAK_DB_PASSWORD}" ] && { echo "ERROR: KEYCLOAK_DB_PASSWORD not set in .env"; exit 1; }
[ -z "${POSTGRES_PASSWORD}" ] && { echo "ERROR: POSTGRES_PASSWORD not set in .env (needed to connect as postgres user)"; exit 1; }

# Escape single quotes for SQL: ' -> ''
PASS_ESC="${KEYCLOAK_DB_PASSWORD//\'/\'\'}"
SQL="ALTER USER keycloak WITH PASSWORD '${PASS_ESC}';"

if command -v psql >/dev/null 2>&1; then
  export PGPASSWORD="${POSTGRES_PASSWORD}"
  psql -h "${POSTGRES_HOST:-172.18.0.1}" -p "${POSTGRES_PORT:-5432}" -U postgres -d postgres -c "$SQL"
  echo "Keycloak DB user password updated (via psql). Restart Keycloak: docker restart keycloak"
else
  # Postgres likely in Docker; find a postgres container and run psql inside it
  PG_CONTAINER=$(docker ps -q --filter "name=postgres" --filter "status=running" 2>/dev/null | head -1)
  if [ -n "$PG_CONTAINER" ]; then
    docker exec -i "$PG_CONTAINER" psql -U postgres -d postgres -c "$SQL" 2>/dev/null || \
    docker exec -e PGPASSWORD="${POSTGRES_PASSWORD}" -i "$PG_CONTAINER" psql -U postgres -d postgres -c "$SQL"
    echo "Keycloak DB user password updated (via docker exec $PG_CONTAINER). Restart Keycloak: docker restart keycloak"
  else
    echo "ERROR: psql not in PATH and no running postgres container found. Install psql or ensure Postgres container is running."
    exit 1
  fi
fi
