# Airflow OAuth: Invalid parameter redirect_uri Fix

**Date**: January 2026  
**Issue**: Keycloak returns **Invalid parameter: redirect_uri** when Airflow redirects to Keycloak. The request showed `redirect_uri=http://airflow.lianel.se/auth/oauth-authorized/keycloak` (wrong scheme and FAB uses `/auth/` prefix).  
**Status**: Fix applied

---

## Cause

1. **Scheme**: FAB's **AuthOAuthView** builds `redirect_uri` with **`url_for(".oauth_authorized", provider=provider, _external=True)`** and passes it to `authorize_redirect()`. So the scheme comes from the **current request**. If the proxy does not send **`X-Forwarded-Proto: https`**, the app sees **http** and sends `redirect_uri=http://...` to Keycloak, which rejects it. Nginx must set **`proxy_set_header X-Forwarded-Proto https`** (literal) for the Airflow `location /`.
2. **Path**: FAB’s auth blueprint uses the **`/auth/`** prefix, so the callback URL is **`/auth/oauth-authorized/keycloak`**, not `/oauth-authorized/keycloak`. Keycloak’s Airflow client must include `https://airflow.lianel.se/auth/oauth-authorized/keycloak` in Valid Redirect URIs (script adds it).

---

## Fixes Applied

### 1. Nginx (`lianel/dc/nginx/config/nginx.conf`)

In the **airflow.lianel.se** HTTPS server block, `location /`:

- Set **`proxy_set_header X-Forwarded-Proto https;`** (literal) so Airflow always sees HTTPS when building redirect_uri.
- Set **`proxy_set_header X-Forwarded-Host $host;`** and **`proxy_set_header X-Forwarded-Port 443;`** for consistency.

### 2. Keycloak Airflow client

- **`lianel/dc/scripts/keycloak-setup/create-airflow-keycloak-client.sh`**: Added **`https://airflow.lianel.se/auth/oauth-authorized/keycloak`** to `redirectUris` (in both create and update branches). The client now allows:
  - `https://airflow.lianel.se/oauth-authorized/keycloak`
  - `https://airflow.lianel.se/auth/oauth-authorized/keycloak`
- Script now uses **`KEYCLOAK_ADMIN_PASSWORD`** from the environment (required); optional **`KEYCLOAK_URL`**, **`KEYCLOAK_ADMIN_USER`**.

---

## Deploy Steps

### On the server (or from a machine with Keycloak admin access)

1. **Sync nginx config** and reload:
   - Copy updated `nginx/config/nginx.conf` to the server.
   - `docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload`

2. **Update Keycloak Airflow client** (add the `/auth/` redirect URI):
   - **Option A**: From the repo on the server, with `.env` (or export) containing `KEYCLOAK_ADMIN_PASSWORD`:
     - `cd /root/hosting-base/lianel/dc` (or your path)
     - `export KEYCLOAK_ADMIN_PASSWORD='...'`  # or source .env
     - `bash scripts/keycloak-setup/create-airflow-keycloak-client.sh`
     - This updates the existing Airflow client with both redirect URIs.
   - **Option B**: In Keycloak Admin UI: **Clients → airflow → Valid redirect URIs** → add **`https://airflow.lianel.se/auth/oauth-authorized/keycloak`** → Save.

3. **Retest**: Open https://airflow.lianel.se, click login, then “Sign in with Keycloak”. Keycloak should accept the redirect_uri and complete the flow.

---

## Source map 404 (Keycloak theme, optional)

The browser may report **Source map error: request failed with status 404** for `auth.lianel.se/resources/.../patternfly.min.css.map`. This is a Keycloak theme asset; it does not affect the OAuth flow or login. You can ignore it or fix it in the Keycloak theme if needed.

---

## datepicker / ab.js Error (separate)

The error **`$(...).datepicker is not a function`** in `ab.js` is a frontend/static issue: Flask-AppBuilder expects jQuery UI (with datepicker). It does not block the OAuth redirect_uri fix. If the login flow works but some UI elements (e.g. date pickers) fail, that can be addressed separately (e.g. ensure FAB’s static assets or Airflow version include jQuery UI).

---

## References

- FAB auth blueprint uses `/auth/` prefix: [Flask-AppBuilder issue #1254](https://github.com/dpgaspar/Flask-AppBuilder/issues/1254)
- Redirect URI scheme behind proxy: ensure `X-Forwarded-Proto: https` and ProxyFix (Airflow has `AIRFLOW__WEBSERVER__ENABLE_PROXY_FIX: 'true'`)
- Keycloak client script: `lianel/dc/scripts/keycloak-setup/create-airflow-keycloak-client.sh`
