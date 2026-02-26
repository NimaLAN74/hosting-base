# Stock Monitoring – Database Migrations (022 & 023)

**Last updated:** February 2026  
**Owner:** Operations / Stock Monitoring

---

## Overview

The Stock Monitoring service uses schema `stock_monitoring` in the shared PostgreSQL database. Two migrations define it:

| Migration | File | Contents |
|-----------|------|----------|
| **022** | `lianel/dc/database/migrations/022_stock_monitoring_schema.sql` | Schema `stock_monitoring`; tables: `watchlists`, `watchlist_items`, `alerts`, `audit_log` |
| **023** | `lianel/dc/database/migrations/023_stock_monitoring_notifications.sql` | Table `stock_monitoring.notifications` (in-app notifications for alert triggers) |

Apply both **in order** before running the backend or alert DAGs.

---

## When to run

- **New environment:** Before first run of the stock-monitoring backend or alert DAG.
- **After cloning repo / new DB:** When the `stock_monitoring` schema or tables are missing.
- **CI:** Workflow `stock-monitoring-ci-deploy.yml` applies 022 and 023 before `cargo test`.
- **Production:** The same workflow applies 022/023 on the production DB during the CD step.

---

## 1. Local / dev (manual)

### Prerequisites

- PostgreSQL running; `psql` and a user that can create schema/tables (e.g. `postgres`).

### Steps

```bash
export PGHOST=127.0.0.1
export PGPORT=5432
export PGUSER=postgres
export PGPASSWORD=postgres
export PGDATABASE=postgres

cd /path/to/hosting-base

psql -v ON_ERROR_STOP=1 -f lianel/dc/database/migrations/022_stock_monitoring_schema.sql
psql -v ON_ERROR_STOP=1 -f lianel/dc/database/migrations/023_stock_monitoring_notifications.sql

echo "Migrations 022 and 023 applied"
```

### Verify

```bash
psql -c "SELECT table_name FROM information_schema.tables WHERE table_schema = 'stock_monitoring' ORDER BY table_name;"
```

Expected: `alerts`, `audit_log`, `notifications`, `watchlist_items`, `watchlists`.

---

## 2. Production (deploy pipeline)

The **Stock Monitoring Backend – CI and Deploy** workflow runs migrations on the remote host in the step **Run migrations 022/023 on production DB**. It uses `.env` on the host for DB connection and creates the stock-monitoring database if needed. No manual step is required if the pipeline succeeds.

To run manually on production, use the same `psql` commands as in section 1 with production host, user, and database from your `.env`.

---

## 3. Rollback

Migrations use `CREATE ... IF NOT EXISTS`. There is no automatic rollback. To drop the schema (destructive, dev only):

```sql
DROP SCHEMA IF EXISTS stock_monitoring CASCADE;
```

Do not run on production unless wiping all watchlists, alerts, and notifications.

---

## Related

- Backend config: `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB`.
- CI: `.github/workflows/stock-monitoring-ci-deploy.yml`.
- Architecture: `lianel/dc/stock-monitoring/docs/ARCHITECTURE-DESIGN.md`.
