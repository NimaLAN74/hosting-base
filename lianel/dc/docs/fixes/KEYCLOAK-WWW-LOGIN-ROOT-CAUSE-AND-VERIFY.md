# Keycloak login on www.lianel.se – root cause and deploy verification

## Root cause (no trial-and-error)

### 1. CSS MIME type "text/html" instead of "text/css"

- **What happens:** Browser requests `https://www.lianel.se/resources/.../patternfly.min.css`. If it gets HTML (e.g. 404 page or app index.html), it reports MIME type "text/html" and blocks the stylesheet.
- **Why it happened:** Keycloak serves theme assets at `/resources/` only when the request **Host** matches its configured frontend host. With `KC_HOSTNAME=https://www.lianel.se/auth`, Keycloak still serves `/resources/` for **Host: auth.lianel.se** (admin host). For **Host: www.lianel.se** without the right path context, Keycloak can return an error page (HTML) for `/resources/...`.
- **Fix (single, correct one):** In nginx, for `location ^~ /resources/` in the **www.lianel.se** server block, set `proxy_set_header Host auth.lianel.se;` so the request to Keycloak uses the host Keycloak expects for `/resources/`. Keep `proxy_pass http://keycloak_backend/resources/;` (no `/auth/` in path).
- **File:** `lianel/dc/nginx/config/nginx.conf` – www server block, first `location ^~ /resources/` (around line 100).

### 2. POST 400 on login-actions/authenticate

- **Root cause:** Nginx forwards `/auth/realms/...` to Keycloak as `/realms/...` (prefix stripped). Keycloak sets cookies with **Path=/realms/lianel/**. The browser then POSTs to **/auth/realms/lianel/login-actions/authenticate**; that path does not start with `/realms/`, so the browser does not send the session cookies → Keycloak returns 400 (invalid/missing session).
- **Fix:** In nginx, for the Keycloak `/auth/` and `/realms/` locations, add **`proxy_cookie_path /realms/ /auth/realms/;`** so Set-Cookie Path is rewritten to `/auth/realms/...`. The browser then sends the cookie when posting to `/auth/realms/...`.
- **File:** `lianel/dc/nginx/config/nginx.conf` – www server block, `location /auth/` and `location ^~ /realms/`.

### 3. Why “changes not taking effect”

- **Nginx:** The running config is the file **mounted** into the container (e.g. `/root/lianel/dc/nginx/config/nginx.conf` → `/etc/nginx/nginx.conf`). Pushing a file elsewhere or not copying to that path means the container never sees the change. Reload only applies the config that’s already in the container.
- **Verification:** After any nginx “deploy”, we must **verify** from the server that the CSS URL returns **200** and **Content-Type: text/css**. No verification ⇒ no guarantee the fix is live.

---

## Deploy verification (mandatory after nginx changes)

Run on the server (or from CI) after copying nginx.conf and reloading nginx. **If this fails, the fix is not active.**

```bash
# On server: config in container
docker exec nginx-proxy grep -A 3 "location ^~ /resources/" /etc/nginx/nginx.conf | head -6
# Must show: proxy_set_header Host auth.lianel.se; and proxy_pass .../resources/

# From anywhere: CSS URL must return 200 and text/css
curl -sI -k "https://www.lianel.se/resources/lbvvu/common/keycloak/vendor/patternfly-v5/patternfly.min.css" | grep -E "HTTP/|Content-Type"
# Must show: HTTP/1.1 200 OK and Content-Type: text/css
```

The script `scripts/monitoring/verify-keycloak-www-resources.sh` (see below) automates this.

---

## Verify in browser (after deploy is verified)

1. **Confirm deploy** using the verification above (curl or script).
2. **Avoid cache:** Use a **new incognito/private window** or **clear site data** for `www.lianel.se` (or at least “Cached images and files”).
3. Open **https://www.lianel.se**, trigger login, check:
   - No “stylesheet … MIME type text/html” in console.
   - Login form submits without 400 (if 400 persists, we need to debug session/redirect_uri separately with evidence).

---

## One-time: ensure repo and server config match

- **Repo:** `lianel/dc/nginx/config/nginx.conf` – www block `location ^~ /resources/` has `proxy_set_header Host auth.lianel.se;` and `proxy_pass http://keycloak_backend/resources/;`.
- **Server:** The file that the nginx container mounts (e.g. `/root/lianel/dc/nginx/config/nginx.conf`) must be updated (copy from repo or pull and copy), then `docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload`.
- **Proof it’s live:** Run the verification curl/script; only then is the change “deployed”.
