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

1. **Option A – Use Client Portal Gateway (Docker)**  
   Deploy and log in to the Gateway, then point the stock-service at it:
   - See **[CLIENT-PORTAL-GATEWAY.md](./CLIENT-PORTAL-GATEWAY.md)** for build, run, login, and `IBKR_API_BASE_URL` + `IBKR_INSECURE_SKIP_TLS_VERIFY` setup.
2. **Option B – TWS / IB Gateway**  
   Run IBKR Trader Workstation or IB Gateway on a machine, log in with the same account that has market data. If the backend uses **api.ibkr.com** (cloud), the “no bridge” message may still appear; in that case use the Gateway (Option A) or check account/region/subscription.
3. Ensure the **session** used by the backend (e.g. the one obtained via `/tickle` and the same LST/cookies) is the one attached to that Gateway/TWS session.
4. Check IBKR docs and account settings for **market data subscriptions** and any “bridge” or gateway requirements for the Client Portal API.
