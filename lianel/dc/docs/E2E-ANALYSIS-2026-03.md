# E2E Analysis – Logs, Cookie/Login, IBKR, Backend→Frontend (Mar 2026)

This document records a step-by-step E2E analysis done **with real logs and tests** (no guessing).

## 1. What the logs showed

### 1.1 oauth2-proxy
- At startup: **502** when doing OIDC Discovery against the configured issuer URL (Keycloak). So if Keycloak is not ready when oauth2-proxy starts, it keeps failing until Keycloak is up; then it succeeds. No fix in code; ensure Keycloak starts first or allow retries.

### 1.2 Keycloak
- **grafana-client**: `LOGIN_ERROR` with `error="cookie_not_found"` when user submitted the login form.
- **grafana-client**: Earlier `Missing parameter: code_challenge_method` (PKCE) — grafana-client was updated so PKCE is not required.
- **frontend-client**: Once `Missing parameter: code_challenge_method` — oauth2-proxy is configured with `--code-challenge-method=S256` and should send PKCE.

### 1.3 Grafana
- Redirects to Keycloak for login; no successful auth in logs when users hit monitoring.lianel.se.

### 1.4 Stock service
- Watchlist background task logs: **`IBKR (status 400): Bad Request: no bridge`** every 60s. So:
  - Tickle (session) succeeds.
  - Snapshot request reaches IBKR; IBKR responds **400 "no bridge"** = Client Portal Gateway not running or not connected.

---

## 2. Cookie / login issue – root cause and fix

**Cause**: For Grafana, the login form was loaded from **auth.lianel.se** (Grafana’s `auth_url` was `https://auth.lianel.se/...`). Keycloak is configured with `KC_HOSTNAME=https://www.lianel.se/auth`, so it sets cookies with **Path=/auth/realms/lianel/** and domain for www. When the user was on **auth.lianel.se**, the request path was **/realms/...**, so the cookie path did not match and the browser did not send the session cookie on POST → Keycloak returned **cookie_not_found**.

**Fix**:
1. **Grafana `auth_url`** must point to **www.lianel.se/auth** so the whole login flow (GET form + POST) happens on the same host/path where Keycloak sets the cookie:
   - `auth_url = https://www.lianel.se/auth/realms/lianel/protocol/openid-connect/auth`
2. **Server path**: The Grafana container on the server mounts **`/root/hosting-base/lianel/dc/monitoring/grafana/grafana.ini`**, not `/root/lianel/dc/...`. So this file was updated on the server and Grafana was restarted. After that, `docker exec grafana grep auth_url /etc/grafana/grafana.ini` shows the correct URL.
3. **auth.lianel.se**: Nginx already has `proxy_cookie_path /auth/realms/ /realms/;` so that any response from Keycloak that sets Path=/auth/realms/ is rewritten to Path=/realms/ for requests to auth.lianel.se. Using www for Grafana avoids path/domain mismatch entirely.

---

## 3. IBKR prices – why they are not shown

- **Backend**: Calls IBKR `/tickle` (session), then `/iserver/marketdata/snapshot`. Snapshot returns **HTTP 400** with body **"no bridge"**.
- **Meaning**: The IBKR Client Portal Gateway (or TWS/IB Gateway) is not running or not connected. The API requires a running Gateway; without it, snapshot always returns 400 no bridge.
- **Backend → frontend**: The backend **does** return HTTP 200 and JSON: `symbols[]` with `price: null` and `error: "IBKR (status 400): Bad Request: no bridge"`. So the pipeline from backend to frontend is correct; the frontend shows “Prices not yet available from provider” when every symbol has an error (see `StockServicePage.js`). So “no prices” is due to **IBKR**, not our backend or frontend.

**What to do**: Run the IBKR Client Portal Gateway (and keep it connected) if you want live prices. See stock-service docs (e.g. 502-WATCHLIST-RUNBOOK.md).

---

## 4. E2E checks performed

1. **Logs**: oauth2-proxy, Keycloak, Grafana, stock-service (watchlist/IBKR) — confirmed messages above.
2. **Login flow**: GET Keycloak auth URL (www.lianel.se/auth/...) → save cookies → POST login form with same cookie. Result: **200** and login page (wrong creds). Keycloak log for that POST showed **`invalid_user_credentials`** (not `cookie_not_found`), so the session cookie **was** received; the failure was only due to wrong username/password. So when the flow uses www.lianel.se/auth, the cookie path is correct.
3. **Watchlist**: `GET https://www.lianel.se/api/v1/stock-service/watchlist` → **200**, body with `symbols[].error` and `price: null` (IBKR no bridge).
4. **Grafana config**: After fix, `auth_url` in container is `https://www.lianel.se/auth/...`.

---

## 5. Scripts

- **`scripts/monitoring/e2e-full-trace-grafana-airflow-stock.sh`**: Can run from anywhere; traces redirects and hits watchlist URL.
- **`scripts/monitoring/e2e-server-full-analysis.sh`**: Intended to run **on the server**; dumps recent container logs, runs GET+POST login flow, calls watchlist API, and checks Grafana `auth_url`. Use this for a full server-side E2E.

---

## 6. Deploy note (Grafana config path)

On the server, the compose project may use **`/root/hosting-base/lianel/dc/`** for mounts (check with `docker inspect grafana`). So `monitoring/grafana/grafana.ini` that the **container** reads is the one under **hosting-base**, not necessarily **lianel**. After changing Grafana config in the repo:

1. Ensure the file that is **actually mounted** on the server has `auth_url = https://www.lianel.se/auth/realms/lianel/protocol/openid-connect/auth`.
2. Restart Grafana: `docker restart grafana`.
3. Verify: `docker exec grafana grep '^auth_url' /etc/grafana/grafana.ini`.
