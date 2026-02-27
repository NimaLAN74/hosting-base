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
5. **Dashboard**: Clicking a symbol opens a gradient area chart over daily close + today’s intraday points (API `GET /api/v1/price-history?symbol=...&days=90`).
