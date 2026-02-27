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
8. **Real-time chart**: With the history modal open, the frontend polls `GET /api/v1/price-history?symbol=...` every 5 seconds so new points (from Redis/Postgres) appear in the chart without closing the modal.

### Troubleshooting: “No history yet” or empty chart

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
5. **Frontend**: Chart modal polls price-history every 5s when open; no extra config.
