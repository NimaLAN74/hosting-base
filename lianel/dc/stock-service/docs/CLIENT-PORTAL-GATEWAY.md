# IBKR Client Portal Gateway – Deploy and Config

When the watchlist returns **IBKR (status 400): Bad Request: no bridge**, the API session is not “bridged” to market data. You can use **api.ibkr.com** (cloud) with OAuth, or run the **Client Portal Gateway** and point the stock-service at it.

## SSO / Automated login (recommended)

You do **not** have to log in to the Gateway in a browser each time. Use **IBeam** so the Gateway stays authenticated with your credentials:

1. **Set credentials** in `.env`:
   ```bash
   IBEAM_ACCOUNT=<your_ibkr_username>
   IBEAM_PASSWORD=<your_ibkr_password>
   ```
   Or use `IBKR_USERNAME` and `IBKR_PASSWORD`; the IBeam compose maps them to `IBEAM_ACCOUNT` / `IBEAM_PASSWORD`.

2. **Start the Gateway with IBeam** (from repo root, e.g. `/root/hosting-base/lianel/dc`):
   ```bash
   docker compose -f docker-compose.infra.yaml -f docker-compose.ibkr-ibeam.yaml up -d ibkr-gateway
   ```
   IBeam runs the Gateway and logs in automatically; it keeps the session alive (maintenance/tickle).

3. **Point stock-service at the Gateway** (see “Point the stock-service at the Gateway” below).

4. **Session cookie for headless API**: The stock-service backend needs the Gateway session cookie to call the API. With IBeam, the Gateway is already logged in. You only need to provide the cookie **once** (or after it expires, ~24h):
   - Open **https://www.lianel.se/ibkr-gateway/** – if IBeam is running you may already be logged in, or log in once.
   - In DevTools → Application → Cookies, copy the **`api`** cookie value.
   - Set in `.env`: `IBKR_GATEWAY_SESSION_COOKIE=<paste value>`, then restart stock-service.

After that, IBeam maintains the Gateway session; you do not log in to the Gateway separately each time.

## When to use the Gateway

- **api.ibkr.com**: Default. No local process. If you get “no bridge”, it may be an account/subscription or region limitation.
- **Client Portal Gateway**: Run the Gateway in Docker. Use **IBeam** (above) for credential-based “SSO” so you don’t log in to the Gateway in a browser each time.

## Deploy the Gateway (Docker)

### Option A: Gateway with IBeam (automated login – recommended)

```bash
docker compose -f docker-compose.infra.yaml -f docker-compose.ibkr-ibeam.yaml up -d ibkr-gateway
```

Requires `IBEAM_ACCOUNT` and `IBEAM_PASSWORD` (or `IBKR_USERNAME` / `IBKR_PASSWORD`) in `.env`. See “SSO / Automated login” above.

### Option B: Plain Gateway (manual or one-time login)

```bash
docker compose -f docker-compose.infra.yaml -f docker-compose.ibkr-gateway.yaml up -d ibkr-gateway
```

Then log in at **https://www.lianel.se/ibkr-gateway/** (see “Gateway web UI” below).

### Gateway web UI (login page)

The Gateway is exposed under the main site so that **one** URL works for login and assets (no wrong domain or MIME issues):

- **https://www.lianel.se/ibkr-gateway/**

All traffic is under `/ibkr-gateway/`: HTML is rewritten so CSS/JS paths and redirects use `/ibkr-gateway/`, and cookies are rewritten so they are valid for the proxy. Use this URL to log in (or to capture the `api` cookie for stock-service). Do **not** use `/sso/` directly; use `/ibkr-gateway/` (or you’ll be redirected there).

### Point the stock-service at the Gateway

In `.env` on the server:

```bash
IBKR_API_BASE_URL=https://ibkr-gateway:5000/v1/api
IBKR_INSECURE_SKIP_TLS_VERIFY=true
```

Restart the stock-service:

```bash
docker compose -f docker-compose.infra.yaml -f docker-compose.stock-service.yaml up -d stock-service
```

### Session cookie (required for headless / stock-service)

The backend needs the Gateway session cookie to call the API. Either:

- **With IBeam**: Log in once at **https://www.lianel.se/ibkr-gateway/** (or capture the cookie when IBeam has already logged in), copy the **`api`** cookie value, set `IBKR_GATEWAY_SESSION_COOKIE=<value>` in `.env`, restart stock-service.
- **Without IBeam**: Log in at **https://www.lianel.se/ibkr-gateway/**, copy the **`api`** cookie, set `IBKR_GATEWAY_SESSION_COOKIE`, restart stock-service. Repeat when the session expires (~24h).

### Verify

- Watchlist should refresh (e.g. every 60s) and return prices if the account has market data and the Gateway is logged in.
- If you still see “no bridge”, ensure the Gateway is running and (with IBeam) authenticated, and that the account has market data subscriptions.

## Files

| Item | Purpose |
|------|--------|
| `docker-compose.ibkr-ibeam.yaml` | Gateway with IBeam – automated login (SSO-style). |
| `docker-compose.ibkr-gateway.yaml` | Plain Gateway image. |
| `stock-service/ibkr-gateway/Dockerfile` | Builds image from official IBKR Client Portal zip (Option B). |
| `stock-service/ibkr-gateway/conf.yaml` | IP allow list for plain Gateway. |

## References

- [IBKR Client Portal API](https://interactivebrokers.github.io/cpwebapi/)
- [IBeam](https://github.com/Voyz/ibeam) – authentication and maintenance for the Gateway
- [502-WATCHLIST-RUNBOOK.md](./502-WATCHLIST-RUNBOOK.md) – “no bridge” and 502 fixes
- [IBKR-WEB-API-REPLACEMENT-PLAN.md](../../docs/status/IBKR-WEB-API-REPLACEMENT-PLAN.md) – api.ibkr.com vs Gateway
