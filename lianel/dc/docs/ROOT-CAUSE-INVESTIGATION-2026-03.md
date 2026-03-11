# Root Cause Investigation – IBKR Prices, Grafana/Airflow Login (Mar 2026)

Deep test performed on the server (logs, curl, Keycloak events) to identify root causes.

---

## 1. IBKR “get price” / watchlist issue

### Observed

- Stock-service logs (every ~60s):  
  `Watchlist IBKR snapshot error response: IBKR (status 400): Bad Request: no bridge`
- Tickle (session) succeeds; the failure is on **snapshot** (market data).

### Root cause

**IBKR returns 400 “Bad Request: no bridge”** from `/iserver/marketdata/snapshot`. This is an **IBKR-side / environment** condition, not an application bug:

- “No bridge” means the Client Portal API session is **not bridged** to market data.
- Typical causes:
  1. **Client Portal Gateway (or TWS) not running** on the machine that “owns” the session used by the API (or not logged in with the account that has market data).
  2. **Account/session** not entitled or not connected for market data (e.g. demo, no data subscription, gateway not logged in with the right account).

### What is working

- OAuth and tickle: backend gets a session from IBKR.
- Backend correctly calls snapshot and parses both success (array) and error (object, e.g. “no bridge”) from IBKR.
- UI shows the error message instead of “no detail”.

### What to do

1. **Run IBKR Client Portal Gateway (or TWS)** and log in with the same account that has market data.
2. Ensure the **session** used by the backend (same LST/cookies as used for tickle) is the one attached to that Gateway/TWS session.
3. In IBKR account settings, confirm **market data subscriptions** and any “bridge”/gateway requirements for the Client Portal API.

See also: `stock-service/docs/502-WATCHLIST-RUNBOOK.md` (IBKR “no bridge” section).

---

## 2. Grafana / Airflow – “Restart login cookie not found” and 400

### Observed (Keycloak logs)

- `LOGIN_ERROR` for `grafana-client`: **`error="cookie_not_found"`**.
- `LOGIN_ERROR` for `grafana-client`: **`error="invalid_request"`, `reason="Missing parameter: code_challenge_method"`** (PKCE enforced, request without PKCE).

So two different failure points:

1. **Before login form**: auth request **without PKCE** → Keycloak rejects with `invalid_request` (Missing `code_challenge_method`) → user never gets a proper login form; callback returns error → user sees “Restart login cookie not found” (oauth2-proxy / app message when callback has an error).
2. **After login form**: if the user ever gets the form and submits, Keycloak can also log **`cookie_not_found`** (e.g. session cookie not sent or expired).

### Root cause (main): PKCE not sent by oauth2-proxy

- Keycloak client **grafana-client** has **PKCE enforced** (`pkce.code.challenge.method`: S256).
- Keycloak log: *“PKCE enforced Client without code challenge method”*.
- **oauth2-proxy** was not sending `code_challenge` and `code_challenge_method` in the authorization request (env `OAUTH2_PROXY_CODE_CHALLENGE_METHOD=S256` can be ignored in some oauth2-proxy versions – see [oauth2-proxy#1852](https://github.com/oauth2-proxy/oauth2-proxy/issues/1852)).
- Result: Keycloak immediately returns `invalid_request` → redirect to app with error → user sees “Restart login cookie not found” or similar; **no successful login flow**.

### Fix applied

- **Force PKCE via CLI** in oauth2-proxy so it is not dependent on the env var:
  - In `docker-compose.oauth2-proxy.yaml`, `oauth2-proxy` service:
  - `command: ["--code-challenge-method=S256"]`
- After deploy, restart the oauth2-proxy container so auth requests to Keycloak include `code_challenge` and `code_challenge_method=S256`. Then Grafana (and any other app using the same proxy) can complete the Keycloak login.

### Cookie path / proxy (already correct)

- Login page was fetched from **www.lianel.se**; Keycloak set cookies with **Path=/auth/realms/lianel/** (correct for `/auth/` context).
- **www** server block already has:
  - `proxy_set_header X-Forwarded-Host` / `X-Forwarded-Port`
  - `proxy_cookie_path /realms/ /auth/realms/` (for any response that still used `/realms/` path).
- So the main blocker for “can’t see any process” / login was **missing PKCE**, not cookie path.

### If `cookie_not_found` still appears after PKCE fix

- Ensure **one tab**, **same host** for the whole flow (e.g. redirect and form post both to `www.lianel.se`).
- Clear site data/cookies for `www.lianel.se` and `monitoring.lianel.se` (and Keycloak domain if different) and retry.
- If it persists, check Keycloak logs at the time of the POST to `login-actions/authenticate` (session expiry, cookie not sent, or path/domain mismatch).

---

## 3. Summary

| Issue | Root cause | Fix |
|-------|------------|-----|
| **IBKR get price** | IBKR returns 400 “no bridge” – Gateway/TWS or session not bridged to market data | Run Client Portal Gateway (or TWS), same account with market data; see runbook |
| **Grafana/Airflow login** | grafana-client/airflow required PKCE; Grafana/FAB may not send it | Run update-grafana-client-no-pkce-required.sh and create-airflow-keycloak-client.sh on server |
| **“Restart login cookie not found”** | Same as above: callback returns error because auth request was rejected (no PKCE) | Apply Grafana and Airflow client fixes above |

---

## 4. Verification (after oauth2-proxy restart)

1. **Grafana**
   - Open `https://monitoring.lianel.se/` (or your Grafana URL).
   - Should redirect to Keycloak login on **www.lianel.se/auth**.
   - After login, Keycloak should redirect back and Grafana should load (no “Restart login cookie not found”).
2. **Keycloak logs**
   - No more `Missing parameter: code_challenge_method` for `grafana-client` (or the client used by oauth2-proxy for that app).
3. **IBKR**
   - Start Client Portal Gateway (or TWS) with the correct account; wait for the watchlist to refresh and show prices or a different error if something else is wrong.
