# IBKR Client Portal Gateway – Deploy and Config

When the watchlist returns **IBKR (status 400): Bad Request: no bridge**, the API session is not “bridged” to market data. You can either use **api.ibkr.com** (cloud) with the same OAuth credentials, or run the **Client Portal Gateway** locally and point the stock-service at it.

## When to use the Gateway

- **api.ibkr.com**: Default. No local process. If you get “no bridge”, it may be an account/subscription or region limitation.
- **Client Portal Gateway**: Run the official Gateway in Docker on the same host (or a reachable host). Log in via the Gateway’s web UI with your IBKR account. The stock-service then calls the Gateway instead of api.ibkr.com; the Gateway provides the “bridge” to market data.

## Deploy the Gateway (Docker)

### 1. Start the Gateway container

From the repo root (e.g. `/root/hosting-base/lianel/dc` or `/root/lianel/dc`):

```bash
docker compose -f docker-compose.infra.yaml -f docker-compose.ibkr-gateway.yaml up -d ibkr-gateway
```

The Gateway listens on port **5000** (HTTPS with a self-signed certificate).

### 2. Log in to the Gateway

Open in a browser (use the server’s hostname or port-forward 5000):

- **https://&lt;server-ip-or-host&gt;:5000**

Log in with the **same IBKR account** that is used for the OAuth consumer key and access token. The Gateway must stay logged in for the “bridge” to work.

### 3. Point the stock-service at the Gateway

In `.env` on the server:

```bash
# Use the Gateway container (same Docker network as stock-service)
IBKR_API_BASE_URL=https://ibkr-gateway:5000/v1/api
# Gateway uses self-signed HTTPS; skip TLS verify for this host
IBKR_INSECURE_SKIP_TLS_VERIFY=true
```

Restart the stock-service:

```bash
docker compose -f docker-compose.infra.yaml -f docker-compose.stock-service.yaml up -d stock-service
```

### 4. Verify

- Watchlist should refresh (e.g. every 60s) and return prices if the account has market data and the Gateway is logged in.
- If you still see “no bridge”, ensure the Gateway web UI shows you as logged in and that the account has market data subscriptions.

## Files

| Item | Purpose |
|------|--------|
| `stock-service/ibkr-gateway/Dockerfile` | Builds image from official IBKR Client Portal zip. |
| `stock-service/ibkr-gateway/conf.yaml` | IP allow list (127.0.0.1, 10.*, 172.*) so Docker and host can connect. |
| `docker-compose.ibkr-gateway.yaml` | Runs the Gateway on `lianel-network` as `ibkr-gateway`. |

## References

- [IBKR Client Portal API](https://interactivebrokers.github.io/cpwebapi/)
- [502-WATCHLIST-RUNBOOK.md](./502-WATCHLIST-RUNBOOK.md) – “no bridge” and 502 fixes
- [IBKR-WEB-API-REPLACEMENT-PLAN.md](../../docs/status/IBKR-WEB-API-REPLACEMENT-PLAN.md) – api.ibkr.com vs Gateway
