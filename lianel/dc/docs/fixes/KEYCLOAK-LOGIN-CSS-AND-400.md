# Keycloak login: CSS MIME type text/html and 400 on authenticate

**Quick fix (recommended):** Run the GitHub Actions workflow **"Fix Keycloak Login (realm + frontend client)"** (Actions → Fix Keycloak Login → Run workflow). It sets realm frontendUrl to `https://www.lianel.se/auth` so login page, theme CSS, and form POST are same-origin (www), and re-applies frontend-client redirect URIs. Then open https://www.lianel.se in a fresh tab and log in again.

---

## 1. Stylesheet MIME type "text/html" (not "text/css")

**Symptom:** Browser reports that `/resources/.../patternfly.min.css` (and other Keycloak theme CSS) was not loaded because its MIME type is "text/html", not "text/css".

**Causes:**

1. **Nginx serving /resources/ from frontend**  
   If the static-asset regex or fallback `location /` serves these requests, the frontend returns `index.html` → `text/html`.  
   **Fix (already in nginx.conf):**
   - `location ^~ /resources/` is the **first** location in the www server block so theme requests go to Keycloak.
   - The static-asset regex excludes `resources/`: `(?!...|resources/)` so `/resources/*.css` never hits the frontend.
   - Ensure the **deployed** nginx config on the server is this version and run `docker exec nginx-proxy nginx -s reload`.

2. **Keycloak returning 404 HTML for theme**  
   Keycloak 26 had regressions where theme resources under `/resources/` returned 404 (HTML error page) → `text/html`.  
   **Fix:** Upgrade Keycloak (e.g. 26.0.2+ / 26.1.0+; we use 26.4.6). If it still happens, check theme configuration in Admin Console (realm → Themes) and that the theme JAR/bundle is present.

## 2. POST login-actions/authenticate → 400 Bad Request

**Symptom:** `POST https://auth.lianel.se/realms/lianel/login-actions/authenticate?...` returns HTTP 400.

**Common causes:**

1. **Session / execution expired**  
   User stayed on the login page too long; the session_code/execution is no longer valid.  
   **Fix:** Refresh the login page (re-open the login URL) and log in again.

2. **Redirect URI / client config**  
   Keycloak rejects the request if `redirect_uri` or client data doesn’t match the client configuration.  
   **Fix:** In Keycloak Admin → realm → Clients → `frontend-client`: ensure **Valid Redirect URIs** includes `https://www.lianel.se/*` (or the exact redirect URI used) and **Web Origins** includes `https://www.lianel.se` (or `+` for valid redirects). Re-run `update-keycloak-frontend-client.sh` if you use it.

3. **Missing or invalid form fields**  
   Username, password, or hidden fields (e.g. execution, session_code) missing or wrong.  
   **Fix:** Don’t alter the login form; retry after a full reload of the login page.

## Quick checks on server

- Nginx config in use:  
  `docker exec nginx-proxy cat /etc/nginx/nginx.conf | head -200`  
  Confirm `location ^~ /resources/` exists and is before other Keycloak locations.
- Reload nginx after config change:  
  `docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload`
- Keycloak version:  
  `docker exec keycloak /opt/keycloak/bin/kc.sh --version`  
  (or check image tag in docker-compose.infra.yaml)
