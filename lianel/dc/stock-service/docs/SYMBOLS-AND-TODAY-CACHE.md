# Symbols and Today Cache

## Why some symbols show price and others don't

The watchlist resolves **symbols → contract IDs (conids)** in two ways:

1. **IBKR `/trsrv/stocks`** (preferred)  
   Before each refresh the backend calls `GET {IBKR_API_BASE_URL}/trsrv/stocks?symbols=AAPL,MSFT,...`.  
   This returns the current conids from IBKR, so we use the **correct** conids for snapshot and history.  
   If this call fails (e.g. network, 401), we fall back to (2).

2. **Hardcoded conids**  
   Fallback map is in `watchlist.rs` (`default_symbol_conids()`).  
   If your symbol’s conid changed or differs per region, **prices can be missing** until trsrv works.

To get prices for all symbols:

- Ensure **IBKR_API_BASE_URL** is correct (e.g. `https://api.ibkr.com/v1/api`).
- If you use a Gateway, trsrv might be under the same base; if trsrv returns 404/401 we still use hardcoded conids.
- Your **IBKR account** must have **market data subscription** for the instruments you request; otherwise snapshot can return “no price …”.
- The watchlist snapshot requests **last (31)** and falls back to **bid/ask (84/86)** when last is empty (delayed / pre-flight).
- **`/history` and `/today`** use **resolved conids** from the latest `/trsrv/stocks` merge (stored in the watchlist cache), not only the static map—so history matches the same contract as live prices when trsrv succeeds.

## Today's intraday cache (Redis)

If **IBKR does not return today’s intraday history**, the app still shows “Today” from:

1. **Live data** – every 60s watchlist refresh updates in-memory points in the browser.
2. **Redis cache** (optional) – if **REDIS_URL** is set on the server, each refresh **writes** the latest price per symbol to Redis.  
   The **Today** chart then uses **GET /api/v1/stock-service/today?symbol=** which reads from Redis and merges with live points.

So:

- **REDIS_URL unset** → Today chart = live only; after a **restart or new deployment** you lose today’s history until the next 60s refresh.
- **REDIS_URL set** → Today chart = cached (Redis) + live; **restarts/deploys do not lose** today’s prices.

Example (same host as Airflow Redis, different DB):

```bash
REDIS_URL=redis://:YOUR_REDIS_PASSWORD@redis:6379/1
```

Use DB **1** (or another free index) if DB 0 is used by Celery.
