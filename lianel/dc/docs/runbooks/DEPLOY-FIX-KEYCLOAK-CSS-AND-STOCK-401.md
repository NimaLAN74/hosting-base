# Deploy: Keycloak login CSS MIME + Stock monitoring 401

If you still see **Keycloak login CSS** (“MIME type text/html”) or **Stock monitoring 401** after code changes, the fixes are in repo but must be **deployed and reloaded** on the server.

---

## 0. One-click: Force update production (recommended)

**GitHub Actions → “Force Update Production (nginx + frontend + stock backend)” → Run workflow.**

This workflow:

1. Syncs `nginx.conf` to the path the nginx container actually uses (`/root/lianel/dc/nginx/config/nginx.conf`) and reloads nginx (fixes Keycloak CSS MIME).
2. Redeploys the frontend container (force-pull `:latest` and recreate).
3. Redeploys the stock-service backend container (force-pull `:latest` and recreate).

**When to use:** After pushing fixes, or when you still see the same errors and suspect the server is running old images or old nginx config.  
**Note:** The workflow pulls **existing** `:latest` images from the registry. To deploy **new** code, run **Deploy Frontend** and **Stock Service Backend** workflows first (to build and push new images), then run **Force Update Production**.

---

## 1. Keycloak login CSS (stylesheet MIME type "text/html")

**Symptom:** Browser console: “The stylesheet https://www.lianel.se/auth/resources/.../patternfly.min.css was not loaded because its MIME type, "text/html", is not "text/css".”

**Cause:** Either `/auth/resources/*` is not handled by the correct nginx location, or the updated nginx config is not active on the server.

**What’s in repo:** In `lianel/dc/nginx/config/nginx.conf` (www.lianel.se server block):

- `location ^~ /auth/resources/` proxies to Keycloak with `Host: auth.lianel.se`, strips `/auth` from the path, and forces `Content-Type` by extension (`.css` → `text/css`, `.js` → `application/javascript`) via `add_header Content-Type $resources_content_type always`.

**Deploy steps:**

1. Copy the current `lianel/dc/nginx/config/nginx.conf` to the **exact path** the nginx container mounts: `/root/lianel/dc/nginx/config/nginx.conf` (see `docker-compose.infra.yaml` volumes). Create the directory if needed: `mkdir -p /root/lianel/dc/nginx/config`.
2. Reload nginx:
   ```bash
   docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload
   ```
   (Use your actual nginx container name if it’s not `nginx-proxy`.)

**Verify:**

```bash
curl -sI "https://www.lianel.se/auth/resources/lbvvu/common/keycloak/vendor/patternfly-v5/patternfly.min.css"
```

- Expect: `HTTP/2 200` and `content-type: text/css` (and no `content-type: text/html`).
- If you get `content-type: text/html` or 502, the new config is not in use or Keycloak is unreachable; fix the config path / reload and that the Keycloak container is running.

---

## 2. Stock monitoring 401 (XHR to /api/v1/stock-service/*)

**Symptom:** Logged in on the main site, but `/stock` shows “Authentication required” and XHRs to `/api/v1/stock-service/me`, `/watchlists`, etc. return **401 Unauthorized**.

**Causes:**

- **No token sent:** Frontend `getToken()` returns null (e.g. token not in memory or localStorage).
- **Token rejected:** Backend expects a different Keycloak issuer (e.g. only `auth.lianel.se` while the main app uses `www.lianel.se/auth`).

**What’s in repo:**

- **Frontend** (`lianel/dc/frontend/src/keycloak.js`): `getToken()` falls back to `localStorage.getItem('keycloak_token')` when `keycloak.token` is empty.
- **Backend** (stock-service): Accepts multiple issuers (primary from `KEYCLOAK_URL` + `KEYCLOAK_ISSUER_ALT`). When a token is sent but invalid, responds with `{"error":"Invalid or expired token","detail":"..."}` instead of “Missing authorization token”.
- **Compose** (`lianel/dc/docker-compose.stock-service.yaml`): `KEYCLOAK_ISSUER_ALT=https://www.lianel.se/auth/realms/lianel` so tokens from the main app’s Keycloak URL are accepted.

**Deploy steps:**

1. **Frontend:** Build and deploy the main frontend (so the updated `keycloak.js` is live).
2. **Stock-monitoring backend:** Rebuild the image, set env (including `KEYCLOAK_ISSUER_ALT`), and redeploy the profile that serves `/api/v1/stock-service/`.
3. On the server, ensure the stock-service service has in its environment:
   - `KEYCLOAK_ISSUER_ALT=https://www.lianel.se/auth/realms/lianel`  
   (or the same value in your `.env` / compose).

**Verify:**

1. Open the site, log in, go to `/stock`.
2. In DevTools → Network, open a failing request to e.g. `/api/v1/stock-service/me` and check the **response body**:
   - `{"error":"Missing authorization token"}` → token not sent; confirm frontend deploy and that you’re logged in (and that `getToken()` / localStorage are used).
   - `{"error":"Invalid or expired token","detail":"..."}` → token sent but invalid; check `KEYCLOAK_ISSUER_ALT` and that the backend was rebuilt/redeployed with multi-issuer support.

---

## 3. One-shot checklist

- [ ] Nginx config from repo copied to server and nginx reloaded (`nginx -t` then `nginx -s reload`).
- [ ] Keycloak container is running and reachable from nginx (no 502 on `/auth/resources/...`).
- [ ] Main frontend redeployed (getToken + localStorage fallback).
- [ ] Stock-monitoring backend rebuilt and redeployed with `KEYCLOAK_ISSUER_ALT` set.
- [ ] Curl to a Keycloak CSS URL shows `content-type: text/css`.
- [ ] From a logged-in session, `/stock` and XHR 401 body checked to distinguish “Missing authorization token” vs “Invalid or expired token”.
