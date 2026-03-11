# OAuth2-Proxy Runbook

## Do we still use it?

**Yes.** Nginx uses oauth2-proxy for session-based protection of several routes via `auth_request /oauth2/auth` (e.g. main app, Airflow, Comp-AI). The React app uses Keycloak JWTs for API calls; oauth2-proxy provides the cookie-based session for browser access to those UIs.

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
