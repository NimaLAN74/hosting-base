# Stock Price History – Space-Efficient Design

## Goal

Persist history of stock price changes for charts and analysis while **minimizing disk use** on a space-limited remote host.

## Why Not Store Every Quote?

- Ingest DAG runs e.g. hourly → ~10 points/day/symbol. 500 symbols × 365 days = **1.8M rows/year**.
- Each row (symbol, timestamp, price, currency) ≈ 40–50 bytes → **~70–90 MB/year**.
- Raw tick storage would be far larger. That is still manageable but not “minimal.”

## Recommended: Daily OHLC Only

Store **one row per symbol per calendar day** with:

- **Open** – first price seen that day  
- **High** – max price that day  
- **Low** – min price that day  
- **Close** – last price seen that day  

(Optional: **Volume** if your provider supplies it and you need it.)

### Space

- **500 symbols × 365 days = 182,500 rows/year**.
- Row: `symbol` (20 bytes) + `trade_date` (4) + `open/high/low/close` (4×4 = 16) ≈ **40 bytes/row** → **~7 MB/year**.
- 2 years ≈ 14 MB, 5 years ≈ 35 MB. Fits easily on a tight host.

### How to Fill It

- When the ingest DAG (or any flow) gets a quote for symbol `S` at time `T`:
  - `trade_date = date(T)` (UTC or exchange TZ).
  - **Upsert** row `(S, trade_date)`:
    - If no row: set open = high = low = close = price.
    - If row exists:  
      - open = keep as is  
      - high = max(high, price)  
      - low = min(low, price)  
      - close = price  

So each new quote during the day only updates that day’s OHLC; no extra rows.

### What You Get

- **Charts**: daily candles for any symbol.
- **History**: “What was the close on date X?” with one row per symbol per day.
- **Trends**: high/low/close over time with minimal storage.

## Alternative: Close-Only (Even Smaller)

If you only need “price at end of day”:

- One row per (symbol, trade_date) with a single **close** column.
- ~**5–6 MB/year** for 500 symbols.
- No open/high/low; less useful for candlestick charts.

## Retention (Optional)

- Keep **daily OHLC** for a fixed window (e.g. last 2 years) and drop older partitions or run a monthly job to delete `trade_date < cutoff`.
- No need to keep hourly or tick history unless you add a separate, short-retention table later.

## Summary

| Strategy           | Rows/year (500 sym) | Approx. size/year | Use case        |
|--------------------|----------------------|-------------------|------------------|
| Every quote        | ~1.8M                | ~70–90 MB         | Avoid on tight host |
| **Daily OHLC**     | **182k**             | **~7 MB**         | **Recommended** |
| Daily close only   | 182k                 | ~5–6 MB           | Minimal, no candles |

**Recommendation (as built):**

1. **Intraday cache** (`stock_monitoring.price_history_intraday`): Each quote during the day is stored as (symbol, observed_at, price). Used for “today” line on the chart.
2. **Daily OHLC** (`stock_monitoring.price_history_daily`): One row per symbol per day (open, high, low, close).
3. **On quote fetch**: Backend appends to `price_history_intraday` (fire-and-forget) so every fetched price is recorded for the current day.
4. **EOD roll** (DAG `stock_monitoring_price_history_roll`, 00:05 UTC): Aggregates intraday rows for past days into OHLC, inserts/updates `price_history_daily`, then deletes those rows from `price_history_intraday`. So only “today” remains in the cache.
5. **Dashboard**: Clicking a symbol opens a gradient area chart over daily close + today’s intraday points (API `GET /api/v1/price-history?symbol=...&days=90`). If the API returns no data, the UI shows the current dashboard price as a single “Now” point when available.
6. **In-memory cache merge**: The price-history API merges the **latest quote from the backend’s in-memory quote cache** into `intraday_today` (when present for that symbol).
7. **Redis intraday cache** (optional): When `REDIS_URL` is set, **each price change** (from `/api/v1/quotes`, `/internal/quotes/ingest`, or provider fetch) is **written** to Redis sorted sets (`stock:intraday:{symbol}:{yyyy-mm-dd}`). The price-history API **reads** today’s points from Redis and merges them with Postgres + in-memory so the chart has the full real-time series. Key TTL 25h. Use DB index 1 if sharing Redis with Airflow (Celery uses 0).
8. **Real-time chart**: With the history modal open, the frontend polls `GET /api/v1/price-history?symbol=...` every 60s so new points (from Redis/Postgres) appear without refreshing too often.

### Deployment and data persistence

- **We do not reset or clear price history on deploy.** The in-memory quote cache is empty after a container restart; that only affects live quotes, not daily/intraday data in Postgres.
- **Migrations 025, 026, 027** use `CREATE TABLE IF NOT EXISTS` and only add GRANTs. They **never** `DROP`, `TRUNCATE`, or `DELETE` from `price_history_daily` or `price_history_intraday`. Re-running them on each deploy is safe and does not remove existing rows.
- **If daily data disappears after each deployment**, the Postgres instance likely has **no persistent volume** (data dir is lost when the Postgres container is recreated). Fix: use a **named volume** or **host-mounted path** for the Postgres data directory. The stock-monitoring deploy only restarts the backend container; it does not touch Postgres.

### Verifying which provider returned each price

The `/api/v1/quotes` response includes a **per-quote `source`** field (when present) indicating which provider returned that price:

- `"finnhub"` – Finnhub (tried first when `STOCK_MONITORING_FINNHUB_API_KEY` or `FINNHUB_API_KEY` is set)
- `"yahoo"` – Yahoo Finance (fallback for symbols Finnhub didn’t resolve)
- `"stooq"` – Stooq (fallback for still-unresolved symbols)
- `"alpha_vantage"` – Alpha Vantage (fallback when `STOCK_MONITORING_DATA_PROVIDER_API_KEY` is set)

Example: call `GET /api/v1/quotes?symbols=SHL.L,ASML.AS,AAPL` and inspect each object in `quotes` for `"source"` to see whether prices come from both Finnhub and Yahoo or only Yahoo.

### Testing the quotes API (root cause for all-unavailable)

To verify the backend without browser auth, call it **on the host** (after SSH) so nginx/auth is bypassed:

```bash
# On the remote host (e.g. after ssh):
curl -s "http://localhost:3003/api/v1/quotes?pairs=AAPL:yahoo,MSFT:yahoo,SHEL:finnhub" | jq .
```

- **200 + quotes with prices**: Backend and providers are working; check frontend/nginx if the UI still shows empty.
- **200 + all quotes with `"source": "unavailable"`**: No provider returned data. Check:
  1. **Env vars**: `STOCK_MONITORING_FINNHUB_API_KEY`, `STOCK_MONITORING_ALPACA_API_KEY` / `STOCK_MONITORING_ALPACA_API_SECRET` in the container (deploy script writes them from GitHub secrets).
  2. **Network**: Container can reach Yahoo/Finnhub/Alpaca (no egress block, DNS OK).
  3. **Logs**: Look for `Yahoo quote fetch failed`, `Finnhub quote fetch failed`, or HTTP 4xx/5xx from providers.
- **400**: Invalid `pairs=` (e.g. typo in provider). Use `yahoo`, `finnhub`, `alpaca`, `stooq`, `alpha_vantage` only.

Integration tests: `quotes_with_pairs_returns_200_and_shape`, `quotes_pairs_after_ingest_returns_cached_by_provider` (prove cache key `symbol|provider` and pairs path).

### Troubleshooting: AAPL:finnhub (or any symbol:provider) returns price 0 / empty

When the API returns `"price": 0.0`, `"stale": true` and a warning like `"No quote for: AAPL:finnhub"`, the backend tried that provider but got no valid price. Check:

1. **Startup logs**: On the host, `docker logs <stock-monitoring-container>` (or journalctl). Look for `Quote providers: Finnhub=yes/...`. If `Finnhub=no`, the env var `STOCK_MONITORING_FINNHUB_API_KEY` (or `FINNHUB_API_KEY`) is not set in the container; fix the deploy secrets and redeploy.
2. **Finnhub request/parse logs**: After a quotes request, look for:
   - `Finnhub quote request failed for AAPL: ...` — network or TLS error.
   - `Finnhub quote parse failed for AAPL (status=401)` — invalid or missing API key (Finnhub returns 401 or a non-JSON body).
   - `Finnhub quote for AAPL: error=...` — Finnhub returned an error message in the JSON body (e.g. invalid token, rate limit).
   - `Finnhub quote for AAPL: no valid price (c=0, pc=123.45)` — API returned but current price is 0 (e.g. outside market hours); backend now falls back to previous close `pc` when available, so if you still see this, both `c` and `pc` were 0 or invalid.
3. **Use previous close**: The backend uses Finnhub’s `pc` (previous close) when `c` (current) is 0 so that symbols still show a price when the market is closed. If you still get 0, the Finnhub response had both zero or the request never succeeded (see logs above).

### Troubleshooting: Finnhub site shows new prices but UI does not

If you see prices changing on Finnhub's site but the app still shows old values, check backend logs (e.g. `docker logs -f <stock-monitoring-container>`) for:

1. **Background refresh** — Every ~60s you should see either `refresh_quotes_cache: refetching N pairs from provider (TTL interval)` (backend calling providers) or `refresh_quotes_cache: no refetch (all X pairs cache fresh)`. If you only ever see `no refetch`, the cache may be considered fresh too long.
2. **What Finnhub returns** — For each symbol: `Finnhub quote SYMBOL: c=... pc=... t=... -> using price=...`. If `c` and `t` stay the same every minute, the Finnhub API may be returning delayed data (e.g. free tier) while the website is real-time.
3. **What we store** — After refresh: `refresh_quotes_cache: updated SYMBOL provider price=...` or `quotes: updated cache SYMBOL provider price=...`. Same price every run means the provider response is unchanged.
4. **Initial load** — The backend runs one full refresh on startup, then every 60s. First request may see stale data for a few seconds.

### Using all quote providers (not just Yahoo)

The backend tries **Finnhub first** (when key is set), then **Alpaca** for unresolved (US stocks), then **Yahoo**, then **Stooq**, then **Alpha Vantage**. If you see `"source": "unavailable"` for US symbols (AAPL, MSFT, etc.) or only Stooq for European symbols, the provider chain failed for those symbols. Common causes: (1) **Deploy never ran** (e.g. CD failed with SSH timeout)—Finnhub/Alpaca env vars never reached the server; (2) **Yahoo fetch failed**—check logs for `Yahoo quote fetch failed`.

1. **GitHub secrets**: In the repo → Settings → Secrets and variables → Actions, add:
   - `STOCK_MONITORING_FINNHUB_API_KEY` (or `FINNHUB_API_KEY`) — from [finnhub.io](https://finnhub.io)
   - `STOCK_MONITORING_ALPACA_API_KEY` and `STOCK_MONITORING_ALPACA_API_SECRET` — from [Alpaca](https://alpaca.markets)
   - Optionally `STOCK_MONITORING_DATA_PROVIDER_API_KEY` (Alpha Vantage), `FINNHUB_WEBHOOK_SECRET`
2. **Redeploy**: Run the “Stock Monitoring Backend – CI and Deploy” workflow; it passes these into the container.
3. **Verify**: Backend logs at startup show `Quote providers: Finnhub=yes/no, Alpaca=yes/no, Yahoo=yes, …`. If Finnhub=no or Alpaca=no, keys did not reach the container.

## Two endpoints: 7-day history vs minute-by-minute today

The UI has two chart modes, each backed by a **separate endpoint**:

| Button        | Endpoint                          | Data source                                      |
|---------------|-----------------------------------|--------------------------------------------------|
| **Last 7 days**  | `GET /api/v1/price-history/daily?symbol=X&days=7`  | `price_history_daily` table (daily bars only)   |
| **Current day**  | `GET /api/v1/price-history/intraday?symbol=X`     | DB + Redis intraday + in-memory cache (minute-by-minute) |

- **Daily** returns only `{ symbol, daily: [...] }` (close_price per calendar day). No intraday data.
- **Intraday** returns only `{ symbol, intraday_today: [...] }` (observed_at, price). No daily data.

The combined `GET /api/v1/price-history?symbol=X&days=7` still exists and returns both; the frontend uses the two separate endpoints and connects each UI button to its endpoint.

**On the remote host** (e.g. after SSH):

```bash
curl -s -H "x-auth-request-user: test" "http://localhost:3003/api/v1/price-history/daily?symbol=SHL.L&days=7" | jq .
curl -s -H "x-auth-request-user: test" "http://localhost:3003/api/v1/price-history/intraday?symbol=SHL.L" | jq .
```

**Response shapes**:

- **daily**: `symbol`, `daily`: array of `{ trade_date, open_price, high_price, low_price, close_price }` — chart uses `trade_date` and `close_price`.
- **intraday**: `symbol`, `intraday_today`: array of `{ observed_at, price }` — chart uses `observed_at` and `price` (+ session points + “Now” from live quotes).

Integration tests: `price_history_returns_daily_and_intraday_arrays` (combined), `price_history_daily_returns_daily_array`, `price_history_intraday_returns_intraday_array`.

### Troubleshooting: “No history yet” or empty chart

- **Empty after refresh or new login**: Session chart points are cleared on refresh/login. Backend history is only from DB and Redis. If daily/intraday are empty, ensure the quotes API is called (dashboard or ingest DAG) so intraday is persisted, and the roll-daily DAG has run; if Redis/DB was restarted, history refills as new data is written.
- **Backend** returns empty when the DB queries fail (missing table or permission). The handler logs a warning and returns 200 with empty arrays so the UI does not 500.
- **Chart shows only one point**: The API merges Postgres intraday + Redis intraday + in-memory latest. Check backend logs: at startup look for `Redis connected for intraday cache` (if you see `REDIS_URL not set` or `Redis connection failed`, Redis is off). On each price-history request you'll see `price_history: symbol=X daily=N intraday_today=M` and when Redis is used `intraday_from_db=N intraday_from_redis=R`. If `intraday_from_redis=0`, no points have been written to Redis yet—trigger a quotes load (open dashboard or run ingest DAG) then open the chart again. Writes log `redis push intraday: K points at <date>`.
- **Check**: Migrations 025 and 026 must be applied on the same DB the app uses. The deploy workflow runs them; if the app user is not `airflow`, grant `SELECT` (and for intraday, `INSERT`) on `stock_monitoring.price_history_daily` and `stock_monitoring.price_history_intraday` to that user.
- **Logs**: Look for `price_history daily (returning empty)` or `persist intraday:` in the stock-monitoring backend logs to see the underlying error (e.g. relation does not exist, permission denied).
- **Grants**: Migration 027 grants SELECT/INSERT (and DELETE for roll) on both price-history tables to `postgres` and `airflow`. If the app user is different, run 027 or manually `GRANT SELECT, INSERT, UPDATE, DELETE ON stock_monitoring.price_history_daily TO <app_user>;` and same for `price_history_intraday` (plus INSERT, DELETE and sequence for intraday).

### Deployment checklist (real-time chart with Redis)

1. **Migrations**: 025, 026, 027 applied (workflow runs them on deploy).
2. **REDIS_URL**: Set in `.env` (or as GitHub secret `REDIS_URL`). Use DB 1 if sharing Redis with Airflow (Celery uses 0). If Redis runs in the Airflow stack, attach its container to `lianel-network` and use the container name: `redis://:${REDIS_PASSWORD}@dc-redis-1:6379/1` (so the stock-monitoring container can resolve the host).
3. **DAGs**: `stock_monitoring_ingest_dag` (hourly 08–17 UTC weekdays) and `stock_monitoring_price_history_roll` (00:05 UTC daily) deployed and enabled.
4. **Backend**: Built with `--features redis` (Dockerfile does this). Container reads REDIS_URL and writes/reads intraday in Redis.
5. **Frontend**: Chart modal polls the active view’s endpoint (daily or intraday) every 60s when open.
