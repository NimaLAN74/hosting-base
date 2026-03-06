# Why 7-day price history can be empty

The "Last 7 days" chart reads from the **price_history_daily** table. That table is **only** filled by the **daily roll** job. If the roll never runs or has no data to aggregate, 7-day history stays empty.

## Data flow

1. **Intraday** – During the day, quotes are written to **price_history_intraday** (and in-memory/Redis) when:
   - The **ingest DAG** runs (hourly 08–17 UTC weekdays): calls `GET /internal/quotes/fetch?symbols=...` → backend fetches from providers and **persists to intraday**.
   - The backend’s **background refresh** runs (every 60s): loads (symbol, provider) from **watchlist_items**, fetches, and persists to intraday.
   - Any call to the quotes API (e.g. from the UI) that fetches from providers also persists to intraday.

2. **Daily roll** – Once per day at **00:05 UTC**, the **stock_monitoring_price_history_roll** DAG runs:
   - Calls `POST /internal/price-history/roll-daily`.
   - Backend **aggregates** from `price_history_intraday` into `price_history_daily` (OHLC per symbol/provider/date) for all dates **before today**.
   - Then **deletes** those rolled rows from `price_history_intraday`.

3. **7-day API** – `GET /api/v1/price-history/daily?symbol=...&provider=...&days=7` reads from **price_history_daily** only.

So 7-day history is empty if:

- The **roll DAG** has never run or often fails (e.g. not deployed, wrong URL, or DB error).
- The **ingest DAG** never runs or fails (e.g. backend unreachable, wrong `STOCK_MONITORING_INTERNAL_URL`, or no symbols in Variable), so **intraday** is never filled for the roll to aggregate.
- Not enough time has passed: the roll only moves data where **trade_date < today**. You need at least one full day of ingest, then the next day’s roll, to see one day in the 7-day chart.

## What to check

1. **Ingest DAG** (`stock_monitoring_ingest`): runs at `0 8-17 * * 1-5` (hourly 08–17 UTC, weekdays). Check Airflow that it runs and succeeds. It calls `/internal/quotes/fetch?symbols=...` (Variable `stock_monitoring_ingest_symbols` or default `ASML.AS,SAP.DE,VOLV-B.ST,SHEL.L`).
2. **Roll DAG** (`stock_monitoring_price_history_roll`): runs at `5 0 * * *` (00:05 UTC daily). Check that it runs and succeeds. It calls `POST /internal/price-history/roll-daily`.
3. **Backend reachable from Airflow**: `STOCK_MONITORING_INTERNAL_URL` must point to the stock backend (e.g. `http://lianel-stock-monitoring-service:3003`) so both DAGs can hit the internal endpoints.
4. **Migrations**: 025 (price_history_daily), 026 (price_history_intraday), 028 (provider column) must be applied so the roll SQL and persist logic match the schema.

## Optional: seed more symbols

Set Airflow Variable **stock_monitoring_ingest_symbols** to a comma-separated list (e.g. `ASML.AS,SAP.DE,VOLV-B.ST,SHEL.L,AAPL,MSFT`) so the ingest DAG fetches (and persists) those symbols. Default is `ASML.AS,SAP.DE,VOLV-B.ST,SHEL.L`.
