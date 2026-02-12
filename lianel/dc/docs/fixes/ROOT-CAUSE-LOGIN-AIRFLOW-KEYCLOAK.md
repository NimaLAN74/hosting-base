# Root Cause Analysis: Login Flow, Keycloak, Airflow

**Date**: January 2026  
**Purpose**: Single source of truth for what broke and why. No try-and-error — fix only identified root causes.

---

## 1. Keycloak 502 Bad Gateway

### Symptom

- `GET https://auth.lianel.se/realms/lianel/...` → **502 Bad Gateway**
- Frontend login fails (can't reach Keycloak)
- Airflow "Sign in with Keycloak" fails (Keycloak unreachable)

### Root cause (confirmed)

**Keycloak container does not start.** Logs show:

```text
FATAL: password authentication failed for user "keycloak"
Failed to obtain JDBC connection
```

Postgres is rejecting the `keycloak` user with the password from the current `.env` (e.g. `KEYCLOAK_DB_PASSWORD`). This often happens after Keycloak was **recreated** (e.g. when running `docker compose up` for another service that depends on Keycloak); the DB user/password in Postgres may have been set differently earlier.

### Quick fix (no SSH)

Trigger the **Sync Nginx (Fix Login)** workflow: GitHub → Actions → **Sync Nginx (Fix Login)** → **Run workflow**. It syncs Keycloak DB password, starts Keycloak, reloads nginx, and sets realm frontendUrl. Re-run whenever `https://www.lianel.se/auth/...` returns **502**.

### Fix (server-side, when you have SSH)

1. On the server, align Postgres with `.env`: run `bash lianel/dc/scripts/maintenance/set-keycloak-db-password-on-server.sh` (from repo root or `/root/hosting-base`).
2. Restart Keycloak: `cd lianel/dc && docker compose -f docker-compose.infra.yaml up -d keycloak`.

See **DIAGNOSIS-KEYCLOAK-502-AND-OLLAMA.md** for details.

---

## 2. Frontend login flow (redirect to app vs redirect to admin)

### Intended behaviour

- User on `https://www.lianel.se` clicks Login → browser goes to **Keycloak** (auth.lianel.se) → after login, redirect **back to** `https://www.lianel.se` (or same path), **not** to `auth.lianel.se/admin/master/console/`.

### Root cause of “redirect to admin”

Two different patterns were tried:

| Pattern | Keycloak URL | Nginx /auth Host | Result |
|--------|--------------|------------------|--------|
| **A (correct)** | `https://auth.lianel.se` | `Host: auth.lianel.se` | Login and token exchange on auth.lianel.se; redirect back to app via **Keycloak client config** (baseUrl/rootUrl, realm frontendUrl). |
| **B (broke it)** | `https://www.lianel.se/auth` (same-origin proxy) | `Host: $host` (www) | Introduced to “fix” CORS; Keycloak then built redirect URLs with www and/or realm frontendUrl wrong → users landed on **auth.lianel.se/admin/master/console/**. |

**Canonical pattern** (Grafana, Airflow, and originally working frontend): **Pattern A**.  
Document: **KEYCLOAK-PATTERN-GRAFANA-AIRFLOW-FRONTEND.md**.

### Fix applied (Jan 2026)

**Observed in browser**: Clicking "Sign In" on www.lianel.se sent the user to **auth.lianel.se/auth/login/** which served **Airflow's** login page, not Keycloak's.

**Root cause**: The **live server's nginx config** was not the same as the repo: both **www.lianel.se/auth/** and **auth.lianel.se** were routing to **Airflow** instead of Keycloak. So Keycloak was never reached.

**Fix 1 – Nginx (server)**: Deploy the repo's **nginx.conf** to the server and reload nginx so that:
- www.lianel.se `location /auth/` and `location ^~ /realms/` proxy to **keycloak_backend**
- auth.lianel.se `location /` proxies to **keycloak_backend**

After deploying and reloading, www.lianel.se/auth/realms/... and auth.lianel.se/realms/... correctly return Keycloak's login page. **Tested in browser**: Sign In now shows "Sign in to Lianel Services" (Keycloak).

**Fix 2 – Frontend (optional)**: Use **https://www.lianel.se/auth** as default Keycloak URL so login goes via www's proxy; if Keycloak returns auth.lianel.se we replace with www.lianel.se/auth in login(). See keycloak.js, docker-compose.frontend.yaml, .env.example.

No frontend or nginx change needed for “redirect to app” — that is achieved by **Keycloak client config** on the server:

- **frontend-client**: `baseUrl` and `rootUrl` = `https://www.lianel.se`; Valid Redirect URIs include `https://www.lianel.se`, `https://www.lianel.se/*`, etc. (script: `update-keycloak-frontend-client.sh`).
- **Realm**: `frontendUrl` = `https://www.lianel.se` (script: `fix-keycloak-https.sh`).

When Keycloak is up, run those scripts on the server if redirect still goes to admin.

### “e.includes is not a function”

This occurred when code assumed `keycloak.createLoginUrl()` always returns a string. In keycloak-js 26.x it can return a Promise. **Current code**: `login()` awaits `createLoginUrl()` and has a `doRedirect(url)` fallback that builds the auth URL manually when `url` is not a string — no `.includes` on a non-string. No further change needed.

---

## 3. Airflow “Invalid parameter: redirect_uri”

### Symptom

- User clicks “Sign in with Keycloak” on Airflow → redirect to Keycloak with `redirect_uri=http://airflow.lianel.se/auth/oauth-authorized/keycloak` → Keycloak returns **400 Invalid parameter: redirect_uri** (wrong scheme and/or URI not in client’s list).

### Root causes

1. **Path**: Flask-AppBuilder (FAB) mounts the OAuth callback under the **`/auth/`** blueprint prefix, so the real callback path is **`/auth/oauth-authorized/keycloak`**, not `/oauth-authorized/keycloak`. Keycloak’s Airflow client must have `https://airflow.lianel.se/auth/oauth-authorized/keycloak` in Valid Redirect URIs (script already adds it).
2. **Scheme**: FAB’s **AuthOAuthView** builds `redirect_uri` with **`url_for(".oauth_authorized", provider=provider, _external=True)`** and passes it to `authorize_redirect()`. So the scheme comes from the **current request** (Flask’s `request`). It does **not** use `OAUTH_REDIRECT_URI` or `remote_app['redirect_uri']` for that call. If the app sees the request as **http** (e.g. nginx not sending `X-Forwarded-Proto: https` or ProxyFix not applied), the redirect_uri sent to Keycloak will be **http** and Keycloak will reject it.

### Fix (repo + deploy)

- **Nginx**: In the **airflow.lianel.se** HTTPS server block, **`location /`** that proxies to Airflow **must** set **`proxy_set_header X-Forwarded-Proto https;`** (literal `https`). Repo **nginx/config/nginx.conf** already has this (line ~1123). If the live server uses a different config, deploy this file and reload nginx.
- **Airflow**: In **config/webserver_config.py** apply **Werkzeug ProxyFix** to `wsgi_app` (when config loads and in FLASK_APP_MUTATOR) so `X-Forwarded-Proto` is trusted and `redirect_uri` is https. Also set **`PREFERRED_URL_SCHEME = "https"`**. Deploy webserver_config.py and restart the Airflow apiserver. (AIRFLOW__WEBSERVER__ENABLE_PROXY_FIX may not be applied in Airflow 3; ProxyFix in webserver_config is required.)
- **webserver_config.py**: `OAUTH_REDIRECT_URI` and `remote_app['redirect_uri']` are set to `https://airflow.lianel.se/auth/oauth-authorized/keycloak` for consistency and for authlib’s token step; they do **not** override the authorize redirect (FAB overrides that with `url_for(..., _external=True)`). Keycloak client script **create-airflow-keycloak-client.sh** already registers both callback URLs.

### Deploy checklist (server)

**Always test after deploy**: `curl -sI https://airflow.lianel.se/` and `curl -sI https://airflow.lianel.se/auth/` (expect 302 to `/oauth2/sign_in` when not logged in, or 200/302 from Airflow when logged in). If you see 503, check oauth2-proxy and Airflow containers and that nginx can reach `airflow-apiserver:8080` and `oauth2-proxy:4180`. If Keycloak returns **400 Bad Request** on the auth URL with `redirect_uri=https://airflow.lianel.se/oauth2/callback`, the **oauth2-proxy** client in Keycloak is missing that redirect URI: run **`scripts/keycloak-setup/fix-oauth2-client.sh`** on the server (with `KEYCLOAK_ADMIN_PASSWORD` from `.env`).

1. **Nginx**: Ensure the repo **nginx/config/nginx.conf** is deployed (e.g. into the nginx container). In the **airflow.lianel.se** server block: **`location /`** must have **`proxy_set_header X-Forwarded-Proto https;`**, and **`location /auth/static/`** must rewrite to `/static/` (so login page static assets are served with correct MIME types). Then: `docker exec <nginx-container> nginx -t && docker exec <nginx-container> nginx -s reload`.
2. **Airflow config**: Ensure **config/webserver_config.py** from the repo is on the server (e.g. in the path mounted as `/opt/airflow/config`). Restart the Airflow apiserver so it picks up config: `docker compose -f docker-compose.airflow.yaml restart airflow-apiserver` (or equivalent).
3. **Test**: Open https://airflow.lianel.se → Login → “Sign in with Keycloak”. The browser should redirect to Keycloak with **`redirect_uri=https://...`** (check the URL bar or DevTools). If it still shows **http**, nginx is not sending `X-Forwarded-Proto` for that request or the wrong server/location is handling it.

---

## 3b. Airflow login page: static assets MIME type (“text/html”) mismatch

### Symptom

- Login page at https://airflow.lianel.se/auth/login/ loads but JS/CSS/fonts from `/auth/static/appbuilder/...` are **blocked**: browser reports “MIME type (‘text/html’) mismatch” and “$ is not defined” (jQuery not loaded).

### Root cause

- The login page references **`/auth/static/...`** (FAB auth blueprint static URL). Requests to `/auth/static/*` hit the auth blueprint; unauthenticated requests are redirected to the login page (HTML), so the browser receives **text/html** instead of application/javascript or text/css.

### Fix (repo + server)

- **Nginx** (airflow.lianel.se): Serve **`/auth/static/appbuilder/`** and **`/auth/static/dist/`** from a **volume** (files copied from the Airflow container), not by proxying to Airflow. Airflow returns HTML/JSON for unauthenticated `/auth/static/` requests → MIME mismatch; rewriting to `/static/` still hit Airflow and returned 404 JSON. So nginx serves the real CSS/JS from files on disk.
- **Copy script**: Run **`scripts/copy-airflow-login-static.sh`** on the server to copy FAB appbuilder and FAB provider dist static from the Airflow apiserver container to **`/root/airflow-login-static`** (or set **`AIRFLOW_LOGIN_STATIC_DIR`**).
- **Compose**: **docker-compose.infra.yaml** mounts **`/root/airflow-login-static:/var/www/airflow-login-static:ro`** in the nginx service so nginx can serve those files with correct MIME types (include **mime.types** in those location blocks).

### Deploy

1. Run **`scripts/copy-airflow-login-static.sh`** on the server (once, and after Airflow image updates).
2. Deploy **nginx/config/nginx.conf** and **docker-compose.infra.yaml** (so nginx has the volume and the `location /auth/static/appbuilder/` and `location /auth/static/dist/` alias blocks).
3. Recreate nginx if the volume was added: **`docker compose -f docker-compose.infra.yaml up -d nginx`**, then **`docker exec nginx-proxy nginx -s reload`**.

---

## 3c. Why Airflow worked before and what changed

### What “worked before”

- **Previously**, Airflow was protected by **OAuth2-proxy** in front of nginx: `location /` had **`auth_request /oauth2/auth`** and **`error_page 401 = @error_401`** redirecting to **`/oauth2/sign_in`**.
- Users hitting https://airflow.lianel.se were **not** sent to Airflow until they had a valid OAuth2-proxy cookie (Keycloak login via oauth2-proxy). They **never saw** the Flask-AppBuilder (FAB) login page, so **`/auth/static/`** was never requested in that flow and there was no MIME mismatch.

### What changed

- OAuth2-proxy protection was **removed** from the Airflow server block (comment in nginx: “OAuth2-proxy protection removed - Airflow handles authentication via Flask AppBuilder OAuth”). The goal was “clean” Keycloak integration with Airflow’s FAB OAuth.
- After removal, unauthenticated users hit Airflow directly and saw the **FAB login page**, which loads **`/auth/static/...`**. The auth blueprint returns HTML (login redirect) for unauthenticated requests → **MIME mismatch** and broken styling. The **redirect_uri** issue (§3) was a separate fix (X-Forwarded-Proto).

### Why OAuth2-proxy in front caused a dead end (Jan 2026)

- With **auth_request** on **`location /`**, unauthenticated users were redirected to **/oauth2/sign_in** → Keycloak → back to Airflow with an oauth2-proxy cookie. **But** oauth2-proxy only checks “user is logged in to Keycloak”; it does **not** create an Airflow (FAB) session. So when the request was proxied to Airflow, FAB saw an unauthenticated user and redirected to **/auth/login/**. User saw “Sign In with keycloak” again → dead end.
- **Fix**: Use **FAB OAuth only** for Airflow (no **auth_request** on **`location /`**). User goes to **/** → Airflow → **/auth/login/** → clicks “Sign In with keycloak” → Keycloak (client **airflow**, redirect_uri **https://airflow.lianel.se/auth/oauth-authorized/keycloak**) → FAB creates session. Ensure Keycloak has the **airflow** client with that redirect URI (run **create-airflow-keycloak-client.sh** on the server if needed).

---

## 4. Summary

| Issue | Root cause | Fix |
|-------|------------|-----|
| **Keycloak 502** | Keycloak fails to start: Postgres auth failed for user `keycloak`. | Server: align Postgres password with `.env` (or reverse); restart Keycloak. No repo change. |
| **Frontend redirect to admin** | Using www.lianel.se/auth and/or wrong Keycloak client/realm config. | Use **auth.lianel.se** (current); redirect to app via frontend-client baseUrl/rootUrl and realm frontendUrl (run scripts on server when Keycloak is up). |
| **Airflow redirect_uri invalid** | FAB builds redirect_uri from request via `url_for(..., _external=True)`; proxy not sending `X-Forwarded-Proto: https`. | Deploy nginx with `proxy_set_header X-Forwarded-Proto https` for airflow.lianel.se `location /`; reload nginx; restart apiserver. |
| **Airflow login static MIME mismatch** | Requests to `/auth/static/*` get login redirect (HTML) instead of static files. | Nginx: `location /auth/static/` rewrite to `/static/` and proxy to Airflow (repo nginx.conf has this). |
| **Airflow “worked before”** | OAuth2-proxy was in front; removal exposed FAB login page and `/auth/static/` MIME issues. | Re-enable OAuth2-proxy for Airflow: `auth_request /oauth2/auth`, `error_page 401 = @error_401`, `location @error_401` redirect to `/oauth2/sign_in` (repo nginx.conf updated). |

**Do not**: Switch frontend to www.lianel.se/auth or change Nginx Keycloak locations to `Host: $host` to “fix” CORS — that pattern caused the redirect-to-admin regression. Resolve CORS via Keycloak **webOrigins** and client config, not by changing the frontend Keycloak URL.

---

## 5. Test steps (after deploy)

Run after each change; stop if a step fails.

1. **Keycloak reachable**  
   `curl -s -o /dev/null -w "%{http_code}" https://auth.lianel.se/realms/lianel/`  
   Expect 200 or 302. If 502, fix Keycloak DB password and restart Keycloak (see §1).

2. **Frontend login**  
   Open https://www.lianel.se → Sign In → complete login on auth.lianel.se.  
   Expect redirect back to https://www.lianel.se (or same path), not auth.lianel.se/admin.

3. **Airflow login**  
   Open https://airflow.lianel.se → Login → "Sign in with Keycloak".  
   Expect redirect to auth.lianel.se, then back to airflow.lianel.se (no "Invalid parameter: redirect_uri").

4. **Airflow login page loads**  
   `curl -s -o /dev/null -w "%{http_code}" https://airflow.lianel.se/auth/login/`  
   Expect 200 (does not require Keycloak to be up).

---

## 6. Actions performed (Jan 2026)

- **Airflow**: Deployed updated `webserver_config.py` (OAUTH_REDIRECT_URI with `/auth/`) to `/root/hosting-base/lianel/dc/config` and `/root/lianel/dc/config`; restarted `dc-airflow-apiserver-1`.
- **Keycloak 502**: Ran `scripts/maintenance/set-keycloak-db-password-on-server.sh` on the server to set Postgres user `keycloak` password from `.env`; restarted Keycloak. Keycloak started successfully.
- **Keycloak admin scripts**: `fix-keycloak-https.sh` and `update-keycloak-frontend-client.sh` failed to get admin token (ensure `KEYCLOAK_ADMIN_PASSWORD` in server `.env` is correct; then run them on the server if redirect-to-app is still wrong).
- **Tests**: Keycloak realm 200, Airflow login page 200, www.lianel.se 200.
