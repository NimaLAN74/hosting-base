# Login Redirect Flow – Analysis and Test Checklist

## Summary

The login redirect must send the user **back to the app** (e.g. `https://www.lianel.se/comp-ai`) after Keycloak auth, never to Keycloak’s domain or a `/login` route.

## End-to-end flow

1. **User** is on `https://www.lianel.se/comp-ai` (or `/`, `/monitoring`, etc.) and clicks “Log in” / “Sign In”.
2. **Frontend** calls `login()` (or `login(true)` to keep current path).
3. **redirect_uri** is built as:
   - `window.location.origin + (pathname + search)` when `redirectToCurrentPath === true`
   - **Exception:** if `pathname.startsWith('/auth')` we use `origin + '/'` so we never use a Keycloak path as callback.
4. **Frontend** redirects the browser to Keycloak:
   - `https://www.lianel.se/auth/realms/lianel/protocol/openid-connect/auth?redirect_uri=<encoded>&client_id=frontend-client&...`
   - Keycloak is reached via same-origin proxy `/auth` → `auth.lianel.se` (nginx).
5. **Keycloak** shows login, user submits. Keycloak redirects the browser to:
   - `redirect_uri?code=...&state=...`
   - So e.g. `https://www.lianel.se/comp-ai?code=...&state=...`.
6. **Frontend** loads again. `keycloak.init()` runs, sees `?code=...`, exchanges code for tokens at `https://www.lianel.se/auth/realms/.../token` (same-origin).
7. **App** clears `?code=&state=` from the URL and keeps the user on the same route (e.g. `/comp-ai`).

## Where “wrong redirect” can happen

| Cause | Symptom | Fix |
|-------|--------|-----|
| **Keycloak client “Valid Redirect URIs”** doesn’t include the app URL | Keycloak error “Invalid redirect uri” or redirect to a Keycloak error page | Add `https://www.lianel.se`, `https://www.lianel.se/`, `https://www.lianel.se/*`, and same for `lianel.se`. Run `update-keycloak-frontend-client.sh` (variables must be expanded). |
| **Keycloak client baseUrl/rootUrl** set to Keycloak domain or empty | After login, user is sent to `auth.lianel.se/admin/master/console/` or Keycloak instead of app | Set `baseUrl` and `rootUrl` to **https://www.lianel.se** for `frontend-client` (run `update-keycloak-frontend-client.sh`). |
| **Realm frontendUrl** set to `https://auth.lianel.se` | Keycloak uses realm frontend URL for “back to app” and redirects users to auth.lianel.se (then admin console) | Set realm attribute `frontendUrl` to **https://www.lianel.se** (run `fix-keycloak-https.sh`). |
| **User opens auth.lianel.se/ directly** | They land on Keycloak root and may end up on admin console | nginx redirects `auth.lianel.se/` → `https://www.lianel.se/`. Admin console remains at `https://auth.lianel.se/admin/master/console/`. |
| **Nginx /auth proxy sends Host: auth.lianel.se** | Keycloak uses that Host to build redirect URLs, so it redirects back to auth.lianel.se after login | Use **Host: $host** in the `/auth/` location on www.lianel.se so Keycloak sees the app host (www.lianel.se) and builds redirect_uri with it. |
| **Keycloak KC_HOSTNAME=auth.lianel.se** | Keycloak uses that as frontend hostname and builds all redirect URLs with auth.lianel.se | Set **KC_HOSTNAME=https://www.lianel.se/auth** and **KC_HOSTNAME_ADMIN=https://auth.lianel.se** in Keycloak so OAuth redirects use the app URL; admin console stays on auth.lianel.se. |
| **Redirect URIs in Keycloak script not expanded** | Keycloak stores literal `"https://${DOMAIN_MAIN}/*"` and rejects real redirect_uri | Use double-quoted JSON and `"https://${DOMAIN_MAIN}/*"` etc. in the script so shell expands variables. |
| **Frontend uses Keycloak path as redirect_uri** | Callback goes to `/auth/...` and app doesn’t handle it like a normal route | In `login()`, if `pathname.startsWith('/auth')` use `origin + '/'` as redirect_uri. |
| **Token exchange redirect_uri mismatch** | Keycloak returns “invalid_grant” or similar when exchanging code | keycloak-js must send the **same** redirect_uri as in the auth request (no `?code=&state=`). keycloak-js does this when we pass redirectUri into `createLoginUrl` and use the same origin/path. |

## Config that must be correct

### 1. Frontend (`keycloak.js` + build)

- **Keycloak URL**: `REACT_APP_KEYCLOAK_URL=https://www.lianel.se/auth` (same-origin proxy). Must **not** be `https://auth.lianel.se` — that makes token/.well-known cross-origin and can break redirect.
- **Build/deploy**: `docker-compose.frontend.yaml` and `.env.example` set this to `https://www.lianel.se/auth` (or `https://${DOMAIN_MAIN}/auth`). Ensure your `.env` or build args do not override it with `KEYCLOAK_URL`/auth.lianel.se.
- **redirect_uri in login()**: `origin + pathname + search`, or `origin + '/'` if path starts with `/auth`.

### 2. Keycloak client `frontend-client`

- **Valid Redirect URIs**: must include at least:
  - `https://www.lianel.se`, `https://www.lianel.se/`, `https://www.lianel.se/*`
  - `https://lianel.se`, `https://lianel.se/`, `https://lianel.se/*`
  - plus any exact paths you use (e.g. `/monitoring`, `/monitoring/*`).
- **Web Origins**: `https://www.lianel.se`, `https://lianel.se` (and optionally `*` if needed).
- **baseUrl / rootUrl**: **https://www.lianel.se** (so Keycloak sends users back to the app after login, not to auth.lianel.se). Run `update-keycloak-frontend-client.sh` to set.

### 3. Nginx

- **www.lianel.se**: `location /auth/` proxies to Keycloak with `Host: auth.lianel.se`.
- No redirect from app URLs to `/login` or to `auth.lianel.se` for normal app navigation.

## Test checklist (manual)

Use this after any change to Keycloak client, frontend auth, or nginx.

- [ ] **Home → Login → Back to home**
  - Open `https://www.lianel.se/`. Click “Sign In” / “Get Started”. Log in. You must land on `https://www.lianel.se/` (or Dashboard), not `/login`, not `auth.lianel.se`.
- [ ] **Comp AI → Login → Back to Comp AI**
  - Open `https://www.lianel.se/comp-ai`. Click “Log in” (if shown). Log in. You must land on `https://www.lianel.se/comp-ai` (or same path with `?code=` stripped), not `/` or `/login`.
- [ ] **Monitoring → Login → Back to Monitoring**
  - Open `https://www.lianel.se/monitoring`. Click “Log In”. Log in. You must land on `https://www.lianel.se/monitoring`.
- [ ] **Profile (unauthenticated) → Login → Back to Profile**
  - Open `https://www.lianel.se/profile` while logged out. You are sent to Keycloak. After login you must land on `https://www.lianel.se/profile`.
- [ ] **No “Invalid redirect uri”**
  - For each of the above, Keycloak must not show “Invalid redirect uri” or “Invalid parameter” related to redirect.
- [ ] **URL bar after login**
  - After redirect, the URL must be the app URL (e.g. `https://www.lianel.se/comp-ai`) and must not contain `/auth` or `auth.lianel.se` as the main domain.
- [ ] **Logout redirect**
  - Log out. You must land on `https://www.lianel.se/` (or configured post-logout URI), not on a Keycloak page.

## Runbook: “502 Bad Gateway on https://www.lianel.se/auth/...”

When `GET https://www.lianel.se/auth/realms/.../auth` returns **502 Bad Gateway**, nginx cannot reach Keycloak. On the **server**:

1. **Keycloak container running**: `docker ps --filter name=keycloak`. If not running: from the dc dir run `docker compose -f docker-compose.infra.yaml up -d keycloak`.
2. **Same Docker network**: nginx must resolve `keycloak:8080`. Check both are on `lianel-network`: `docker network inspect lianel-network --format '{{range .Containers}}{{.Name}} {{end}}'` should list keycloak and nginx-proxy.
3. **Keycloak responding**: from a container on the same network, `curl -s -o /dev/null -w "%{http_code}" http://keycloak:8080/` should return 200 or 302.
4. **Nginx upstream**: in `location /auth/`, `proxy_pass` must point at `http://keycloak:8080/`. Reload: `docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload`.

## Runbook: “Redirects to auth.lianel.se/admin/master/console/ and can’t get to landing”

Do these in order on the **server** (where Keycloak and nginx run):

1. **Keycloak hostname = app URL** (fixes "redirects to auth.lianel.se after login")
   In Keycloak env (e.g. `docker-compose.infra.yaml`): set **KC_HOSTNAME=https://www.lianel.se/auth** and **KC_HOSTNAME_ADMIN=https://auth.lianel.se**, then **restart Keycloak**.

2. **Nginx /auth proxy: send app Host**
   On **www.lianel.se**, in `location /auth/` use **`proxy_set_header Host $host;`** (not `Host auth.lianel.se`). Deploy nginx config and reload nginx.

3. **Set realm frontendUrl and frontend-client rootUrl**  
   From the repo or a machine that can reach Keycloak admin:

   ```bash
   cd /path/to/lianel/dc/scripts/keycloak-setup
   APP_FRONTEND_URL=https://www.lianel.se ./fix-keycloak-https.sh
   ./update-keycloak-frontend-client.sh
   ```

4. **Redirect auth.lianel.se/ to the app**  
   Deploy the updated nginx config that adds:
   - `location = / { return 302 https://www.lianel.se/; }` on `auth.lianel.se`  
   Then reload nginx: `docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload` (or your usual reload).

5. **Use the app URL for login**  
   Users should open **https://www.lianel.se** and use “Sign In” / “Log in” there. They must not start from https://auth.lianel.se when logging into the app.

## Script fixes applied

- **`lianel/dc/scripts/keycloak-setup/update-keycloak-frontend-client.sh`**  
  Redirect URIs use variable expansion. **baseUrl** and **rootUrl** are set to `https://${DOMAIN_MAIN}` (the app) so Keycloak does not send users to auth.lianel.se/admin after login.

- **`lianel/dc/scripts/keycloak-setup/fix-keycloak-https.sh`**  
  Realm attribute **frontendUrl** is set to `https://www.lianel.se` (or `$APP_FRONTEND_URL`) so Keycloak’s “frontend” is the app, not auth.lianel.se.

- **`lianel/dc/nginx/config/nginx.conf`**  
  - **www.lianel.se** `location /auth/`: **`proxy_set_header Host $host;`** so Keycloak sees the app host and builds redirect_uri with www.lianel.se.  
  - **auth.lianel.se** `location = /`: returns `302 https://www.lianel.se/` so visiting https://auth.lianel.se/ sends users to the app. Admin console stays at https://auth.lianel.se/admin/master/console/.

- **`lianel/dc/docker-compose.infra.yaml`** (Keycloak)  
  **KC_HOSTNAME=https://www.lianel.se/auth** and **KC_HOSTNAME_ADMIN=https://auth.lianel.se** so OAuth redirects use the app URL; admin uses auth.lianel.se.

## Frontend safeguard applied

- **`lianel/dc/frontend/src/keycloak.js`**  
  In `login()`, if `pathname.startsWith('/auth')` we set `redirectUri = origin + '/'` so the callback never targets a Keycloak path.
