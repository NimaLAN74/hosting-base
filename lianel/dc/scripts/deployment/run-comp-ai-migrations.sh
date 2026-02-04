#!/usr/bin/env bash
# Run Phase 4/5/6 comp_ai migrations (009–018) on the DB used by comp-ai-service.
# 018 ensures demo seed data (controls, requirements, mappings, tests) so the UI is not empty.
# Run from lianel/dc (where .env and database/migrations live).
# Usage: from lianel/dc: bash scripts/deployment/run-comp-ai-migrations.sh
# Also invoked by deploy-comp-ai-service GH Action on production DB.
#
# On the server, the DB user must have permission to create schema and objects in
# POSTGRES_DB. Set COMP_AI_MIGRATION_USER (and optionally COMP_AI_MIGRATION_PASSWORD)
# in .env to use a DDL-capable user (e.g. postgres); otherwise POSTGRES_USER is used.

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
# Use COMP_AI_MIGRATION_USER if set (e.g. postgres), else POSTGRES_USER (must have DDL on DB)
export PGUSER="${COMP_AI_MIGRATION_USER:-${POSTGRES_USER:-airflow}}"
export PGDATABASE="${POSTGRES_DB:-lianel_energy}"
# For migration user, prefer COMP_AI_MIGRATION_PASSWORD if set, else POSTGRES_PASSWORD
export PGPASSWORD="${COMP_AI_MIGRATION_PASSWORD:-${POSTGRES_PASSWORD:?POSTGRES_PASSWORD not set}}"

MIGRATIONS_DIR="database/migrations"
for f in "$MIGRATIONS_DIR/009_create_comp_ai_schema.sql" \
         "$MIGRATIONS_DIR/010_comp_ai_controls_evidence.sql" \
         "$MIGRATIONS_DIR/011_comp_ai_seed_soc2.sql" \
         "$MIGRATIONS_DIR/012_comp_ai_grants.sql" \
         "$MIGRATIONS_DIR/013_comp_ai_seed_iso27001.sql" \
         "$MIGRATIONS_DIR/014_comp_ai_remediation_tasks.sql" \
         "$MIGRATIONS_DIR/015_comp_ai_seed_gdpr.sql" \
         "$MIGRATIONS_DIR/016_comp_ai_expand_frameworks.sql" \
         "$MIGRATIONS_DIR/017_comp_ai_control_tests.sql" \
         "$MIGRATIONS_DIR/018_comp_ai_ensure_demo_seed.sql"; do
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

echo "Running comp_ai migrations (009–018)..."
for f in "$MIGRATIONS_DIR/009_create_comp_ai_schema.sql" \
         "$MIGRATIONS_DIR/010_comp_ai_controls_evidence.sql" \
         "$MIGRATIONS_DIR/011_comp_ai_seed_soc2.sql" \
         "$MIGRATIONS_DIR/012_comp_ai_grants.sql" \
         "$MIGRATIONS_DIR/013_comp_ai_seed_iso27001.sql" \
         "$MIGRATIONS_DIR/014_comp_ai_remediation_tasks.sql" \
         "$MIGRATIONS_DIR/015_comp_ai_seed_gdpr.sql" \
         "$MIGRATIONS_DIR/016_comp_ai_expand_frameworks.sql" \
         "$MIGRATIONS_DIR/017_comp_ai_control_tests.sql" \
         "$MIGRATIONS_DIR/018_comp_ai_ensure_demo_seed.sql"; do
  echo "  $f"
  run_psql "$f"
done
echo "Done."
