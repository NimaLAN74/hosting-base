# Stock Monitoring Backend

Rust API for stock exchange monitoring and analysis. **MVP: EU markets.**

**Tests:** Unit tests in `src/app.rs` (currency/symbol helpers); integration tests in `tests/` (config, DB, health, status, quotes, openapi, auth, watchlist CRUD, alert CRUD). Run with Postgres and `POSTGRES_*` (or `DATABASE_URL`).  
**CI:** `.github/workflows/stock-monitoring-ci-deploy.yml` – on push/PR: migrations 022/023/024, `cargo test`, build, image push; on main/master: deploy and prod migrations.

## Endpoints (as built)

- `GET /health` – health check (no auth)
- `GET /api/v1/status` – service info + DB connected
- `GET /api/v1/quotes?symbols=...` – quotes (provider + cache)
- `GET /api/v1/symbols/providers`, `GET /api/v1/symbols?provider=&q=|exchange=` – symbol list from DB (synced by DAG; no live API calls)
- `GET /api-doc`, `GET /swagger-ui` – OpenAPI docs
- `GET /api/v1/me`, watchlists, alerts, notifications – protected (JWT or `x-auth-request-user`)
- `POST /internal/alerts/evaluate` – internal; used by Airflow DAG
- `POST /internal/symbols/refresh?provider=finnhub` – internal; syncs Finnhub symbols into DB (used by DAG `stock_monitoring_symbol_sync`)

## Run locally

```bash
export POSTGRES_PASSWORD=postgres  # or your DB password
PORT=3003 cargo run
```

Apply migrations 022, 023, and 024 first; see [STOCK-MONITORING-MIGRATIONS.md](../../docs/runbooks/STOCK-MONITORING-MIGRATIONS.md). After deploy, run the symbol sync DAG (or `POST /internal/symbols/refresh?provider=finnhub`) once so the symbol list is populated.

## Tests

```bash
export POSTGRES_HOST=127.0.0.1 POSTGRES_PASSWORD=postgres POSTGRES_DB=postgres
cargo test
```

CI runs all tests against a Postgres service after applying 022/023.

## Environment

- `PORT` – default 3003
- `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB` – for DB (or `DATABASE_URL`)
- `KEYCLOAK_URL`, `KEYCLOAK_REALM` – JWT validation via JWKS (defaults: https://auth.lianel.se, lianel)
- `STOCK_MONITORING_QUOTE_PROVIDER`, `STOCK_MONITORING_DATA_PROVIDER_API_KEY`, `STOCK_MONITORING_QUOTE_CACHE_TTL_SECONDS` – quote provider and cache
- `STOCK_MONITORING_FINNHUB_API_KEY` or `FINNHUB_API_KEY` – when set, [Finnhub.io](https://finnhub.io) is used as a quote source (real-time quote API); fallback remains Yahoo/Stooq/Alpha Vantage

## Runbooks

- Migrations: [STOCK-MONITORING-MIGRATIONS.md](../../docs/runbooks/STOCK-MONITORING-MIGRATIONS.md)
- Symbols: [STOCK-MONITORING-SYMBOLS.md](../../docs/runbooks/STOCK-MONITORING-SYMBOLS.md)
- Alert debugging: [STOCK-MONITORING-ALERT-DEBUG.md](../../docs/runbooks/STOCK-MONITORING-ALERT-DEBUG.md)

## Shared vs specific

- **Auth:** Keycloak (shared infra).
- **Database:** Schema `stock_monitoring` in shared Postgres; migrations 022, 023, 024 in `lianel/dc/database/migrations/`. Table `symbols` is filled by DAG `stock_monitoring_symbol_sync` (or manual refresh).
- **Quotes:** On-demand from provider: Finnhub (when API key set), Yahoo, Stooq, Alpha Vantage; in-memory cache; ingest DAG warms cache on schedule.
