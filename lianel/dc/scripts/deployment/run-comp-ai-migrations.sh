#!/usr/bin/env bash
# Run Phase 4 comp_ai migrations (009, 010, 011) on the DB used by comp-ai-service.
# Run from lianel/dc (where .env and database/migrations live).
# Usage: from lianel/dc: bash scripts/deployment/run-comp-ai-migrations.sh

set -e
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DC_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$DC_DIR"

if [ -f .env ]; then
  set -a
  . ./.env
  set +a
fi

export PGHOST="${POSTGRES_HOST:-172.18.0.1}"
export PGPORT="${POSTGRES_PORT:-5432}"
export PGUSER="${POSTGRES_USER:-airflow}"
export PGDATABASE="${POSTGRES_DB:-lianel_energy}"
export PGPASSWORD="${POSTGRES_PASSWORD:?POSTGRES_PASSWORD not set}"

MIGRATIONS_DIR="database/migrations"
for f in "$MIGRATIONS_DIR/009_create_comp_ai_schema.sql" \
         "$MIGRATIONS_DIR/010_comp_ai_controls_evidence.sql" \
         "$MIGRATIONS_DIR/011_comp_ai_seed_soc2.sql"; do
  if [ ! -f "$f" ]; then
    echo "Missing $f" >&2
    exit 1
  fi
done

run_psql() {
  local f="$1"
  if command -v psql >/dev/null 2>&1; then
    psql -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d "$PGDATABASE" -f "$f" -v ON_ERROR_STOP=1
  else
    docker run --rm \
      -v "$(pwd)/$MIGRATIONS_DIR:/migrations:ro" \
      -e PGPASSWORD \
      -e PGHOST \
      -e PGPORT \
      -e PGUSER \
      -e PGDATABASE \
      --network host \
      postgres:15-alpine \
      psql -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -d "$PGDATABASE" -v ON_ERROR_STOP=1 -f "/migrations/$(basename "$f")"
  fi
}

echo "Running comp_ai migrations (009, 010, 011)..."
for f in "$MIGRATIONS_DIR/009_create_comp_ai_schema.sql" \
         "$MIGRATIONS_DIR/010_comp_ai_controls_evidence.sql" \
         "$MIGRATIONS_DIR/011_comp_ai_seed_soc2.sql"; do
  echo "  $f"
  run_psql "$f"
done
echo "Done."
