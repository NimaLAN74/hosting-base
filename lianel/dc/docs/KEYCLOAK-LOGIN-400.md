# Keycloak login: 400 Bad Request on login-actions/authenticate

When the login form POST to `https://www.lianel.se/auth/realms/lianel/login-actions/authenticate` returns **400 Bad Request**, the browser is not sending the session cookie Keycloak expects, or the session/CSRF state is invalid.

## What we already do

- **Cookie path**: Nginx rewrites Keycloak’s `Set-Cookie` path from `/realms/` to `/auth/realms/` so cookies are sent when posting to `www.lianel.se/auth/realms/...` (`proxy_cookie_path` in the `www` server’s `/auth/` and `/realms/` locations).
- **Proxy headers**: Keycloak is run with `KC_PROXY_HEADERS: xforwarded` and Nginx sends `X-Forwarded-Host`, `X-Forwarded-Proto`, etc.

## What to try first

1. **Clear cookies** for `www.lianel.se` and `lianel.se`, then open the app again (e.g. Grafana at monitoring.lianel.se) and sign in. Stale or wrong-path cookies often cause 400.
2. **Use a single tab** for the login flow; avoid opening the login page in multiple tabs or copying the URL to another tab.
3. **Retry quickly** after the login page loads; long delays can expire the session/execution and lead to 400.

## If 400 persists

- Check **Keycloak logs** on the server for the exact error (e.g. “Cookie not found”, “Invalid state”):
  ```bash
  docker logs keycloak --tail 100 2>&1 | grep -i "error\|400\|cookie"
  ```
- Ensure **grafana-client** (and any other client used for that flow) has the correct **Valid redirect URIs** in Keycloak (e.g. `https://monitoring.lianel.se/*`, `https://monitoring.lianel.se/login/generic_oauth`).
- Keycloak is configured with **KC_HOSTNAME: auth.lianel.se**. Login is reached via **www.lianel.se/auth**; cookie path rewriting is in place so this is normally fine. If you ever expose Keycloak under a different host/path, align `KC_HOSTNAME` / `KC_HOSTNAME_PATH` and Nginx proxy/cookie settings.
