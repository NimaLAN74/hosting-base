# Stock Service (backend)

Minimal Rust API for **SSO verification** and **IBKR integration**. Replaces the previous stock backend (watchlists, quotes, alerts removed).

## Endpoints

- `GET /health` – health check (no auth)
- `GET /api/v1/status` – service info + `ibkr_oauth_configured` (no auth)
- `GET /api/v1/me` – current user from Keycloak token (protected)

## Run locally

```bash
# No DB required. Keycloak URL/realm have defaults.
PORT=3003 cargo run
```

Optional: set `KEYCLOAK_URL`, `KEYCLOAK_REALM`, and `KEYCLOAK_ISSUER_ALT` for your environment. For IBKR OAuth, set the vars in `.env.example` (consumer key, access token, secret, PEM paths).

## Tests

```bash
cargo test
```

No Postgres required. Tests cover `/health`, `/api/v1/status`, and `/api/v1/me` (401 without token).

## Environment

- `PORT` – default 3003
- `KEYCLOAK_URL`, `KEYCLOAK_REALM` – JWT validation via Keycloak userinfo (same as profile-service). Optional `KEYCLOAK_ISSUER_ALT` for www.lianel.se/auth issuer.
- IBKR OAuth (optional): see `.env.example` and [IBKR-OAUTH-KEYCLOAK-INTEGRATION.md](../../docs/status/IBKR-OAUTH-KEYCLOAK-INTEGRATION.md).

## Frontend

The main app serves the stock page at `/stock` (see `frontend/src/stock-service/StockServicePage.js`). It calls `/api/v1/stock-service/me` and `/api/v1/stock-service/status` to verify SSO.
