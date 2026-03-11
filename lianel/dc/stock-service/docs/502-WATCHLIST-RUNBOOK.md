# 502 Bad Gateway on /api/v1/stock-service/watchlist

When the browser or E2E gets **502 Bad Gateway** in a few tens of ms on:
- `GET /api/v1/stock-service/watchlist`
- `GET /api/v1/stock-service/health`
- `GET /api/v1/stock-service/status`

it means **Nginx could not get a response from the stock-service backend** (upstream down or unreachable).

## Fix on the server

1. **Ensure the stock-service container is running**

   From the server (or via SSH):

   ```bash
   cd /root/lianel/dc   # or /root/hosting-base/lianel/dc
   bash scripts/maintenance/ensure-stock-service-up.sh
   ```

   Or use the full stack script:

   ```bash
   bash scripts/maintenance/ensure-all-services-up-on-server.sh
   ```

2. **If the container keeps exiting, check logs**

   ```bash
   docker logs lianel-stock-service --tail 100
   ```

   Common causes: missing `.env` or IBKR PEM paths, port in use, OOM.

3. **Confirm Nginx can reach the backend**

   - Backend container must be on the same Docker network as Nginx (`lianel-network`).
   - Compose file `docker-compose.stock-service.yaml` attaches the service to `lianel-network`.

## After deploy (CI/CD)

If deploy did not run (e.g. SSH timeout from GitHub Actions), the running server may still have an old or stopped container. Run `ensure-stock-service-up.sh` on the server after fixing deploy/SSH so the new image is deployed and the container is up.

---

## IBKR "Bad Request: no bridge" (status 400)

When the watchlist shows **IBKR (status 400): Bad Request: no bridge**, the Client Portal API is rejecting the market-data request because there is no active “bridge” to market data.

**Typical causes**

- **Client Portal Gateway (or TWS) not running** on the machine that holds the session used for the API (or the session is not the one that has market data).
- **Account / session** not allowed or not connected for market data (e.g. demo, no data subscription, or gateway not logged in with an account that has data).

**What to do**

1. Ensure **IBKR Client Portal Gateway** (or TWS) is running and logged in with the same account that has market data.
2. Ensure the **session** used by the backend (e.g. the one obtained via `/tickle` and the same LST/cookies) is the one attached to that Gateway/TWS session.
3. Check IBKR docs and account settings for **market data subscriptions** and any “bridge” or gateway requirements for the Client Portal API.
