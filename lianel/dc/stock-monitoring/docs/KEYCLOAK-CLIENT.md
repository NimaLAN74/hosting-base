# Keycloak client for Stock Monitoring (Step 1.7)

**Backend** validates JWTs via **JWKS** (public keys). No backend client or client secret is required.

**Frontend** (Phase 5) needs a way for users to log in and obtain a Bearer token to call the stock-monitoring API.

## Option A – Reuse existing client (recommended for MVP)

- Use the same Keycloak client as the main frontend (e.g. `www.lianel.se` or `frontend`).
- Add redirect URI for the stock UI if served on a different path, e.g. `https://www.lianel.se/stock/*`.
- Users log in once; the same token is sent as `Authorization: Bearer <token>` to `/api/v1/stock-monitoring/*`.

## Option B – Dedicated client for stock-monitoring UI

1. In Keycloak Admin → Clients → Create:
   - **Client ID:** `stock-monitoring` (or `stock-monitoring-frontend`)
   - **Client authentication:** ON if the frontend is a confidential client (e.g. server-side flow); OFF for public SPA.
   - **Valid redirect URIs:** `https://www.lianel.se/stock/*` (or your stock UI origin).
   - **Web origins:** `https://www.lianel.se` (or your domain).

2. If confidential: copy the client secret and use it in the frontend (e.g. env) for OAuth2 flows. Backend does **not** need this secret (it uses JWKS).

## Verification

- Backend accepts any JWT from the same realm/issuer (e.g. `https://auth.lianel.se/realms/lianel`) that validates with the realm’s JWKS.
- Ensure `KEYCLOAK_URL` and `KEYCLOAK_REALM` in the stock-monitoring backend match your Keycloak instance.
