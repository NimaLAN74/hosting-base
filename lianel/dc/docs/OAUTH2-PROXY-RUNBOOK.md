# OAuth2-Proxy Runbook

## Do we still use it?

**Yes.** Nginx uses oauth2-proxy for session-based protection of several routes via `auth_request /oauth2/auth` (e.g. main app, Airflow, Comp-AI, Grafana). The React app uses Keycloak JWTs for API calls; oauth2-proxy provides the cookie-based session for browser access to those UIs.

### "Restart login cookie not found" or never reaching the login form (PKCE)

If users are sent to Keycloak but then see an error (e.g. "Restart login cookie not found") **without** ever seeing the login form, or the redirect returns immediately with an error, the usual cause is **Keycloak rejecting the auth request** because **PKCE is required** but oauth2-proxy did not send `code_challenge` and `code_challenge_method`. Keycloak logs show: `error="invalid_request"`, `reason="Missing parameter: code_challenge_method"`.

**Fix**: oauth2-proxy must send PKCE. In `docker-compose.oauth2-proxy.yaml` the oauth2-proxy service has `command: ["--code-challenge-method=S256"]` so PKCE is forced via CLI (the env var `OAUTH2_PROXY_CODE_CHALLENGE_METHOD` can be ignored in some versions). Restart oauth2-proxy after changing compose so the new command is used.

### "Restart login cookie not found" after Keycloak login (cookie / redirect)

If users see this on Grafana or Airflow **after** signing in with Keycloak (e.g. after submitting the login form), nginx may not be passing **X-Forwarded-Host** and **X-Forwarded-Port** to oauth2-proxy. oauth2-proxy needs these to build the correct redirect URI and set cookies for the same host that receives the callback. Ensure every `location /oauth2/` and `location = /oauth2/auth` in `nginx.conf` includes:

- `proxy_set_header X-Forwarded-Host $host;`
- `proxy_set_header X-Forwarded-Port $server_port;`

Then reload nginx and have users sign in again (clear site cookies for the affected host if needed).

## Check container logs (root cause)

Always check logs when login fails:

- **oauth2-proxy**: `docker logs oauth2-proxy --tail 50`  
  - `ERROR: ... failed to discover OIDC configuration: unexpected status "502"` → oauth2-proxy cannot reach the OIDC issuer URL (e.g. https://www.lianel.se/auth/realms/lianel) at startup. Keycloak may be down or slow; nginx returns 502. **Fix**: Start Keycloak first, wait until healthy, then start oauth2-proxy. It will retry and eventually succeed once Keycloak is up.
- **Grafana**: `docker logs grafana --tail 50`  
  - `error=invalid_request errorDesc="Missing parameter: code_challenge_method"` → Keycloak client grafana-client requires PKCE; run `scripts/keycloak-setup/update-grafana-client-no-pkce-required.sh` on the server.
  - `[oauth.login] Login provider denied login request` → same (Keycloak rejected the auth request).
- **Keycloak**: `docker logs keycloak --tail 80`  
  - `PKCE enforced Client without code challenge method` → client (e.g. grafana-client) has PKCE required; remove requirement via Keycloak admin or the update script.
  - `error="cookie_not_found"` → session cookie not sent on login form POST. Use auth_url that matches nginx cookie path (e.g. Grafana auth_url = https://www.lianel.se/auth/...) and/or add `proxy_cookie_path /auth/realms/ /realms/` on auth.lianel.se if login is on auth.lianel.se.

## Why it was restarting

1. **Compose variable names (fixed in repo)**  
   The compose file used `OAUTH2_CLIENT_ID`, `OAUTH2_CLIENT_SECRET`, `OAUTH2_COOKIE_SECRET` while `.env.example` and the server `.env` use `OAUTH2_PROXY_CLIENT_ID`, `OAUTH2_PROXY_CLIENT_SECRET`, `OAUTH2_PROXY_COOKIE_SECRET`. The compose file is now aligned to the `OAUTH2_PROXY_*` names.

2. **Empty secrets on the server**  
   The container was getting empty values because the server `.env` had self-references with empty defaults, e.g.:
   - `OAUTH2_PROXY_CLIENT_SECRET=${OAUTH2_PROXY_CLIENT_SECRET:-}`
   - `OAUTH2_PROXY_COOKIE_SECRET=${OAUTH2_PROXY_COOKIE_SECRET:-}`  
   So the effective values were empty and oauth2-proxy exited with "missing setting: client-secret / cookie-secret", then Docker restarted it in a loop.

## Fix on the server

1. **Get the oauth2-proxy client secret from Keycloak**  
   In Keycloak Admin → realm **lianel** → Clients → **oauth2-proxy** → Credentials, copy the client secret.

2. **Generate a cookie secret** (must be 16, 24, or 32 bytes; 32 hex chars = 16 bytes):
   ```bash
   openssl rand -hex 16
   ```

3. **Set the variables in the server `.env`** (e.g. `/root/lianel/dc/.env` or `/root/hosting-base/lianel/dc/.env`):
   ```bash
   OAUTH2_PROXY_CLIENT_ID=oauth2-proxy
   OAUTH2_PROXY_CLIENT_SECRET=<paste Keycloak client secret>
   OAUTH2_PROXY_COOKIE_SECRET=<paste output of openssl rand -base64 32>
   ```
   Remove any line that sets these to `${...:-}` so the values are literal. If Keycloak is reached via the main site (e.g. `https://www.lianel.se/auth`), set:
   ```bash
   OAUTH2_PROXY_OIDC_ISSUER_URL=https://www.lianel.se/auth/realms/lianel
   ```
   so the issuer matches OIDC discovery.

4. **Restart oauth2-proxy** (from the same directory as the `.env`):
   ```bash
   cd /root/lianel/dc   # or /root/hosting-base/lianel/dc
   docker compose -f docker-compose.oauth2-proxy.yaml up -d oauth2-proxy
   ```

5. **Verify**  
   - `docker ps` shows oauth2-proxy as "Up" (not "Restarting").  
   - `docker logs oauth2-proxy --tail 20` shows no "invalid configuration" errors.

## If you want to stop the restart loop before fixing secrets

To avoid log spam while you prepare the secrets, you can stop the container and disable restart:

```bash
docker stop oauth2-proxy
docker update oauth2-proxy --restart=no
```

After setting the env vars and fixing `.env`, run `docker compose -f docker-compose.oauth2-proxy.yaml up -d oauth2-proxy` again and set restart policy back if needed (the compose file uses `restart: unless-stopped`).
