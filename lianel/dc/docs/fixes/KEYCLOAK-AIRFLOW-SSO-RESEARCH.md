# Keycloak + Airflow SSO: Deep Research and Fix Summary

**Date**: January 2026  
**Purpose**: Single source of truth for why Airflow SSO worked before, what broke, and what is required for it to work now.

---

## 1. What “worked before”

- **OAuth2-proxy in front of Airflow**: nginx had `auth_request /oauth2/auth` and `error_page 401 = @error_401` on `location /` for airflow.lianel.se.
- Users never hit the Airflow (FAB) login page: they authenticated via **oauth2-proxy** → Keycloak (client **oauth2-proxy**) → cookie set by oauth2-proxy → nginx passed request to Airflow. Airflow saw an already-authenticated request (or no auth; behaviour depended on headers).
- So “Airflow SSO” was effectively **oauth2-proxy SSO**; FAB OAuth (Keycloak client **airflow**) was not used.

---

## 2. What changed and why it “doesn’t work now”

- **OAuth2-proxy removed from Airflow** (by design): so that Airflow uses **FAB OAuth** with Keycloak client **airflow** and creates a real FAB session (DAG access, roles, etc.).
- After removal:
  1. Unauthenticated users hit **Airflow** → **FAB login page** (`/auth/login/`).
  2. That page loads **`/auth/static/...`**; the auth blueprint returned HTML for unauthenticated requests → **MIME mismatch** (fixed by nginx serving static from copied files).
  3. User clicks “Sign in with Keycloak” → FAB redirects to Keycloak with **`redirect_uri`**. FAB builds it with **`url_for(..., _external=True)`** → scheme came from the **request**. Behind nginx, the request was **http** → **`redirect_uri=http://...`** → Keycloak returned **400 Invalid parameter: redirect_uri** (fixed by **ProxyFix** + `X-Forwarded-Proto` in webserver_config.py).
  4. Callback path is **`/auth/oauth-authorized/keycloak`** (FAB blueprint prefix `/auth/`). Keycloak client **airflow** must have this in Valid Redirect URIs (script **create-airflow-keycloak-client.sh** adds it).
  5. After Keycloak login, Keycloak redirects to **https://airflow.lianel.se/auth/oauth-authorized/keycloak?code=...&state=...**. FAB exchanges the code for tokens and fetches **userinfo** from Keycloak. If **AIRFLOW_OAUTH_CLIENT_SECRET** is missing or wrong, or userinfo URL/response is wrong, login can fail (500 or “User info does not have username or email”).

---

## 3. Required pieces (checklist)

| Piece | Where | Status |
|-------|--------|--------|
| **Nginx** | `proxy_set_header X-Forwarded-Proto https` for airflow.lianel.se `location /` and `/auth/` | ✅ Repo nginx.conf |
| **ProxyFix** | webserver_config.py: wrap `wsgi_app` with `ProxyFix(x_proto=1, x_host=1)` so `redirect_uri` is https | ✅ Applied at config load + FLASK_APP_MUTATOR |
| **FAB config path** | `AIRFLOW__FAB__CONFIG_FILE: /opt/airflow/config/webserver_config.py` | ✅ docker-compose.airflow.yaml |
| **Callback path** | Keycloak client **airflow** has `https://airflow.lianel.se/auth/oauth-authorized/keycloak` in Valid Redirect URIs | ✅ create-airflow-keycloak-client.sh |
| **Client secret** | `AIRFLOW_OAUTH_CLIENT_SECRET` in .env on server (value from Keycloak client **airflow**) | ⚠️ Server .env only; run create-airflow-keycloak-client.sh and add secret to .env |
| **Userinfo** | OAUTH_USER_INFO_ENDPOINT + remote_app `userinfo_endpoint` for Keycloak userinfo URL | ✅ webserver_config.py |
| **Static assets** | Nginx serves `/auth/static/appbuilder/` and `/auth/static/dist/` from volume (copy-airflow-login-static.sh) | ✅ Repo + run script on server |
| **No auth_request on Airflow** | airflow.lianel.se `location /` does **not** use `auth_request` (FAB OAuth only) | ✅ Repo nginx.conf |

---

## 4. Why it worked before vs now

- **Before**: “SSO” was oauth2-proxy; Airflow did not do OAuth. No FAB login page, no redirect_uri, no callback.
- **Now**: Full FAB OAuth with Keycloak (client **airflow**). Requires: https redirect_uri (ProxyFix + nginx), correct callback URL in Keycloak, client secret in .env, userinfo endpoint, static assets served by nginx.

---

## 5. Fixes applied in repo

1. **webserver_config.py**
   - `PREFERRED_URL_SCHEME = "https"`.
   - `OAUTH_REDIRECT_URI` and `remote_app['redirect_uri']` = `https://airflow.lianel.se/auth/oauth-authorized/keycloak`.
   - `OAUTH_USER_INFO_ENDPOINT` and `remote_app['userinfo_endpoint']` = Keycloak userinfo URL.
   - **ProxyFix** applied to `wsgi_app` when config loads (current_app) and in **FLASK_APP_MUTATOR**.
   - **before_request** hook to force `wsgi.url_scheme` and `X-Forwarded-Proto` for `/auth/` and when header is already https.

2. **Nginx**
   - airflow.lianel.se: `X-Forwarded-Proto https` for locations proxying to Airflow.
   - `/auth/static/appbuilder/` and `/auth/static/dist/` served from volume (no proxy to Airflow for these).

3. **Keycloak**
   - Script **create-airflow-keycloak-client.sh**: client **airflow**, confidential, redirect URIs include `/auth/oauth-authorized/keycloak`. Server must run this and set **AIRFLOW_OAUTH_CLIENT_SECRET** in .env.

4. **E2E test**
   - **scripts/monitoring/e2e-test-airflow-keycloak-login.sh**: hits `/auth/login/`, then `/auth/login/keycloak`, checks redirect_uri is https and Keycloak returns 200.

---

## 6. PKCE: Missing parameter code_challenge_method (exact issue from flow trace)

**Symptom**: When following the flow, Keycloak immediately redirects back to Airflow with:
`error=invalid_request&error_description=Missing+parameter%3A+code_challenge_method`

**Cause**: The Keycloak client **airflow** had `pkce.code.challenge.method`: "S256" in attributes, so Keycloak requires PKCE (code_challenge and code_challenge_method) in the authorize request. FAB/authlib does not send PKCE by default.

**Fix**: Remove the PKCE requirement from the Airflow client in Keycloak. In **create-airflow-keycloak-client.sh** the `attributes` block with `pkce.code.challenge.method` was removed so Keycloak does not require PKCE for this confidential client. **Run the script on the server** (with KEYCLOAK_ADMIN_PASSWORD) to update the client: `bash scripts/keycloak-setup/create-airflow-keycloak-client.sh`

---

## 7. Deploy and test

1. **Deploy webserver_config.py** (pipeline or script):  
   `bash lianel/dc/scripts/deployment/deploy-airflow-webserver-config.sh`
2. **On server**: ensure **AIRFLOW_OAUTH_CLIENT_SECRET** is in .env (from `create-airflow-keycloak-client.sh` output). Restart apiserver if you change .env.
3. **Run E2E test**:  
   `bash lianel/dc/scripts/monitoring/e2e-test-airflow-keycloak-login.sh`
4. **Manual test**: Open https://airflow.lianel.se → Login → “Sign in with Keycloak” → log in on Keycloak → expect redirect back to Airflow and FAB session (DAGs, etc.).

If login still fails after Keycloak (e.g. 500 or “User info” error): check apiserver logs; ensure client secret is set and Keycloak user has username/email (FAB requires one of these from userinfo).

---

## 8. “The request to sign in was denied” (after Keycloak login)

**Symptom**: You reach the Keycloak login page, sign in, then Airflow shows “The request to sign in was denied” instead of logging you in.

**Cause**: The failure happens in the **token exchange**: FAB calls Keycloak’s token endpoint with the authorization `code`; Keycloak or Authlib raises, and FAB shows this generic message. Common causes:

1. **Wrong or empty client secret** – Keycloak returns 401/400 on the token endpoint; FAB catches the exception and shows “denied”.
2. **redirect_uri mismatch** – The `redirect_uri` sent in the token request must match exactly what was used in the authorize request. If the callback request is seen as `http` (e.g. missing `X-Forwarded-Proto` for `/auth/`), the token request would send `redirect_uri=http://...` and Keycloak would reject it.
3. **Keycloak token endpoint error** – Invalid grant, expired code, or other Keycloak error (see apiserver logs).

**Steps to fix**

1. **Get the real error** (on the server):
   - Reproduce: open Airflow → Login → Sign in with Keycloak → log in on Keycloak → when “denied” appears, immediately check logs.
   - Apiserver logs:  
     `docker logs <airflow-apiserver-container> 2>&1 | tail -200 | grep -A5 -i "oauth\|authoriz\|token\|denied\|Error"`
   - FAB logs the exception as: **“Error authorizing OAuth access token: …”** — the rest of the line (or next lines) is the actual exception (e.g. 401 from Keycloak, invalid_client, invalid_grant).

2. **Verify client secret in the container**:
   ```bash
   docker exec <airflow-apiserver-container> env | grep AIRFLOW_OAUTH
   ```
   - You must see `AIRFLOW_OAUTH_CLIENT_SECRET=<non-empty value>`.
   - If it’s empty or missing: add it to the **host** `.env` (same dir as `docker-compose.airflow.yaml`), then restart:
     ```bash
     cd /path/to/lianel/dc
     # Edit .env: AIRFLOW_OAUTH_CLIENT_SECRET=<value from create-airflow-keycloak-client.sh>
     docker compose -f docker-compose.airflow.yaml up -d --force-recreate
     ```

3. **Refresh the client secret from Keycloak** (if you’re unsure it’s correct):
   ```bash
   cd /path/to/lianel/dc
   export KEYCLOAK_ADMIN_PASSWORD='...'   # or source .env
   bash scripts/keycloak-setup/create-airflow-keycloak-client.sh
   ```
   Copy the printed **Airflow Client Secret** into `.env` as `AIRFLOW_OAUTH_CLIENT_SECRET=...`, then restart the apiserver (step 2).

4. **Confirm redirect_uri** – Nginx must send `X-Forwarded-Proto https` for `location /auth/` (see §3). Our nginx.conf already sets this for airflow.lianel.se. If you use a different proxy, ensure it sets the same header for the callback URL.

Once the real exception is visible in the logs (step 1), you can target the fix (wrong secret, redirect_uri, or Keycloak configuration).
