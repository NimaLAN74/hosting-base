# Stock Monitoring Backend

Rust API for stock exchange monitoring and analysis. **MVP: EU markets.**

**TDD:** Integration tests in `tests/` (config + DB pool). Run with Postgres available and `POSTGRES_PASSWORD` set.  
**CI:** Every push to stock-monitoring (or migration 022) runs migration, `cargo test`, build, push image, and deploy. **HTTPS** for external access once Nginx location is added (task 1.6).

## Endpoints (planned)

- `GET /health` – health check (no auth)
- `GET /api/v1/status` – service info + DB connected
- Watchlists, alerts, and quote endpoints to be added per function list.

## Run locally

```bash
export POSTGRES_PASSWORD=postgres  # or your DB password
PORT=3003 cargo run
```

## Tests (TDD)

```bash
export POSTGRES_HOST=127.0.0.1 POSTGRES_PASSWORD=postgres POSTGRES_DB=postgres
cargo test
```

CI runs these against a Postgres service container after applying migration 022.

## Environment

- `PORT` – default 3003
- `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB` – required for DB.
- `KEYCLOAK_URL`, `KEYCLOAK_REALM` – for JWT validation via JWKS (defaults: https://auth.lianel.se, lianel). No client secret needed.

## Shared vs specific

- **Auth:** Keycloak (shared infra).
- **Database:** Schema `stock_monitoring` in shared Postgres; migrations in `lianel/dc/database/migrations/`.
- **Data feeds:** EU market data provider TBD (e.g. Alpha Vantage, Polygon, or EU-focused provider).
