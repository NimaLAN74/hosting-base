# Stock monitoring /stock and API 401 analysis

## What was checked

1. **GET https://www.lianel.se/stock**
   - Response: **302** → redirect to `https://www.lianel.se/stock/`
   - After redirect: **200**, body is the main app’s `index.html` (React SPA shell: Lianel World, `main.*.js`, `root` div). So the page itself is served correctly.

2. **GET https://www.lianel.se/api/v1/stock-monitoring/me** (and other protected endpoints)
   - Without `Authorization` header: **401**, body `{"error":"Missing authorization token"}`.
   - So when the browser calls these from the /stock UI, either no token is sent or the backend rejects the token.

## Root causes and fixes

### 1. Token not sent (getToken() null)

- **Cause:** `getToken()` only returned `keycloak.token`. After client-side navigation to `/stock` or before init restores from localStorage, `keycloak.token` can be empty even though a token exists in `localStorage`.
- **Fix:** `getToken()` in `lianel/dc/frontend/src/keycloak.js` now falls back to `localStorage.getItem('keycloak_token')` so the stock UI (and any caller) sends the token when it’s stored.

### 2. Token sent but wrong issuer (backend 401)

- **Cause:** Main app uses Keycloak at `https://www.lianel.se/auth`. Keycloak may put `iss` in the JWT as `https://www.lianel.se/auth/realms/lianel`. The stock-monitoring backend only accepted the primary issuer from `KEYCLOAK_URL` (e.g. `https://auth.lianel.se/realms/lianel`), so it rejected the token and still returned “Missing authorization token” (no distinct message for “invalid token”).
- **Fix:**
  - Backend accepts **multiple issuers**: primary from `KEYCLOAK_URL` + optional `KEYCLOAK_ISSUER_ALT` (e.g. `https://www.lianel.se/auth/realms/lianel`). Validation tries each until one succeeds.
  - Backend now returns **401 with a clear body** when a token is present but invalid: `{"error":"Invalid or expired token","detail":"..."}` so it’s distinguishable from “no token”.
  - `docker-compose.stock-monitoring.yaml` sets `KEYCLOAK_ISSUER_ALT` so tokens from the main app’s Keycloak URL are accepted.

### 3. Nginx

- Nginx already forwards `Authorization` via `proxy_set_header Authorization $http_authorization` for `/api/v1/stock-monitoring/`. No change needed.

## Deploy

- Deploy frontend (keycloak.js getToken change).
- Rebuild and deploy stock-monitoring backend (multi-issuer + 401 body).
- Ensure backend env has `KEYCLOAK_ISSUER_ALT` (e.g. in compose or server .env) when the main app uses `https://www.lianel.se/auth` for Keycloak.
