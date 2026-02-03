# Keycloak login fix – verification results

## E2E test (run this to verify login flow)

From the repo root:

```bash
bash lianel/dc/scripts/monitoring/e2e-test-www-login-flow.sh
```

The script checks:

1. GET auth URL with PKCE → 200 or 302, then Keycloak login HTML (not React).
2. Login page has form and Keycloak markers.
3. **Form action** must use **www.lianel.se** or **/auth/** (not auth.lianel.se), or login POST returns 400.
4. Theme CSS request returns `Content-Type: text/css`.

If the test fails on “form action uses auth.lianel.se”, ensure the server uses `docker-compose.infra.yaml` with `KC_HOSTNAME: https://www.lianel.se/auth` and restart Keycloak (e.g. run **Sync Nginx (Fix Login)** workflow).

## What was fixed (in repo)

- **nginx** (`lianel/dc/nginx/config/nginx.conf`): For `www.lianel.se`, the three Keycloak locations (`^~ /resources/`, `/auth/`, `^~ /realms/`) now use `proxy_set_header Host $host;` instead of `Host auth.lianel.se`, so Keycloak builds form action and theme URLs with `www.lianel.se` (same-origin).
- **Fix Keycloak Login** workflow: Sets realm `frontendUrl` and frontend-client redirect URIs.

## Verification run (from CI/local)

| Test | Expected | Result |
|------|----------|--------|
| `GET https://www.lianel.se/auth/realms/lianel/` | 200, JSON | ✅ 200, realm JSON |
| `GET https://www.lianel.se/` | 200 | ✅ 200 |
| `GET https://www.lianel.se/auth/` | 302 or 200 | ✅ 302 |
| `GET https://auth.lianel.se/realms/lianel/` | 200 | ✅ 200 |
| `GET https://www.lianel.se/auth/realms/lianel/protocol/openid-connect/auth?...` (login page) | Keycloak HTML, form action `www.lianel.se` | ❌ **200, 750 bytes – React index.html** (frontend app) |
| Theme CSS e.g. `www.lianel.se/resources/.../patternfly.min.css` | 200, `text/css` | ❌ 404 or not `text/css` |

## Conclusion

- **Realm and basic auth URLs** via `www.lianel.se/auth/realms/lianel/` work.
- **Login page and theme** are **not** coming from Keycloak on the live site: a request to the auth endpoint returns the **React app** (index.html, 750 bytes), so `/auth/` is still being served by the frontend on the server, not by Keycloak.

So either:

1. The **updated nginx config was not applied** on the server (copy path, volume mount, or reload issue), or  
2. Another layer (e.g. different nginx, load balancer) is routing `www.lianel.se/auth/` to the frontend.

## What to do to fix login

### Option A: Run workflows (recommended)

1. **Sync Nginx (Fix Login)**  
   GitHub Actions → **Sync Nginx (Fix Login)** → Run workflow.  
   This copies `nginx.conf` from the repo to the server and reloads nginx so `/auth/`, `/realms/`, `/resources/` go to Keycloak.

2. **Fix Keycloak Login (realm + frontend client)**  
   Run this too so realm `frontendUrl` and redirect URIs are set.

3. Open **https://www.lianel.se** in a **new tab** (or clear site data) and try logging in.

### Option B: On the server

1. **Confirm nginx config and reload**
   - On the host:  
     `grep -A2 "location /auth/" /root/lianel/dc/nginx/config/nginx.conf`  
     You should see `proxy_pass http://keycloak_backend/` and `proxy_set_header Host $host;`.
   - Reload:  
     `docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload`
2. **If the file is old or wrong**: run **Sync Nginx (Fix Login)** workflow, or copy the repo’s `lianel/dc/nginx/config/nginx.conf` to `/root/lianel/dc/nginx/config/nginx.conf` on the server.

After that, re-check:

- `curl -sI "https://www.lianel.se/auth/realms/lianel/protocol/openid-connect/auth?client_id=frontend-client&redirect_uri=https%3A%2F%2Fwww.lianel.se%2F&response_type=code&scope=openid"`  
  should return a Keycloak HTML response (large body), not the React index.html.
