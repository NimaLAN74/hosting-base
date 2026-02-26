# Stock Monitoring – Adding Symbols (Watchlists & Ingest)

**Last updated:** February 2026  
**Owner:** Stock Monitoring / Ops

---

## Overview

Symbols in the Stock Monitoring MVP follow **EU markets** conventions. This runbook explains:

1. **Symbol format** (and optional MIC) for watchlists and alerts.
2. **Adding symbols via the API** (watchlists and alerts).
3. **Ingest DAG** (when implemented): where symbol lists and MICs are configured.

---

## 1. Symbol format

- **Format:** `SYMBOL` or `SYMBOL.MIC` (exchange suffix).
- **Examples:**
  - `AAPL` – US (supported by quote provider).
  - `SAP.DE` – XETRA (Germany).
  - `ASML.AS` – Euronext Amsterdam.
  - `VOLV-B.ST` – Nasdaq Stockholm (backend maps `.ST` → `.STO` for Alpha Vantage).
- **Validation:** Backend normalizes and validates symbols when adding to watchlists or alerts; invalid format returns 400.

### Common EU MICs (reference)

| MIC | Exchange / market |
|-----|--------------------|
| XETR | XETRA (Germany) |
| XAMS | Euronext Amsterdam |
| XPAR | Euronext Paris |
| XLON | London Stock Exchange |
| XSTO | Nasdaq Stockholm |

Backend accepts optional `mic` on watchlist items and alerts; it is stored for display and future use. Quote lookup is primarily by **symbol** (including suffix like `.DE`, `.ST`).

---

## 2. Adding symbols via the API

Users add symbols through the **frontend** (Dashboard, Watchlists, Alerts). The backend exposes:

- **Watchlists:** `POST /api/v1/watchlists` (create), then `POST /api/v1/watchlists/:id/items` with body `{"symbol":"SAP.DE","mic":"XETR"}` (optional `mic`).
- **Alerts:** `POST /api/v1/alerts` with body e.g. `{"symbol":"ASML.AS","condition_type":"above","condition_value":400.0,"enabled":true}`.

No runbook step is required for normal use; this section is for API-level debugging or scripts.

### Example (curl with auth)

```bash
# After obtaining a Bearer token (Keycloak)
curl -s -X POST "https://<host>/api/v1/stock-monitoring/api/v1/watchlists" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"EU Tech"}' 
# Returns watchlist id

curl -s -X POST "https://<host>/api/v1/stock-monitoring/api/v1/watchlists/<WATCHLIST_ID>/items" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"symbol":"ASML.AS"}'
```

---

## 3. Ingest DAG – symbol list

The **ingest DAG** (`stock_monitoring_ingest`) warms the backend quote cache on a schedule by calling `GET /api/v1/quotes?symbols=...`. The backend fetches from its providers (Yahoo/Stooq/Alpha Vantage) and caches; the alerts DAG runs a few minutes after to evaluate price alerts.

- **Where to configure symbols:** Set the Airflow Variable **`stock_monitoring_ingest_symbols`** (comma-separated), e.g. `ASML.AS,SAP.DE,VOLV-B.ST,SHEL.L,AAPL`. If unset, the DAG uses a default EU set (ASML.AS, SAP.DE, VOLV-B.ST, SHEL.L).
- **Provider format:** Use the same symbol format as the backend (e.g. `.ST` for Stockholm; backend maps to `.STO` for Alpha Vantage when needed). When **Finnhub** is configured (`STOCK_MONITORING_FINNHUB_API_KEY` or `FINNHUB_API_KEY`), the backend tries Finnhub first; Finnhub accepts US symbols (e.g. AAPL) and many international tickers (see [Finnhub docs](https://finnhub.io/docs/api/quote)).
- **Schedule:** Ingest runs hourly 08–17 UTC on weekdays; the alerts DAG (`stock_monitoring_alerts`) runs at :05 past the hour and waits for this DAG’s success.
- **Internal URL:** The DAG calls the backend using `STOCK_MONITORING_INTERNAL_URL` (default `http://lianel-stock-monitoring-service:3003`). Ensure Airflow workers can reach this URL.

---

## 4. Troubleshooting

| Issue | Check |
|------|--------|
| Symbol rejected (400) | Ensure format is valid (alphanumeric + optional `.MIC`). See backend validation in `app.rs` (symbol normalization). |
| No quote for symbol | Provider may not support that exchange or symbol; check backend logs and provider docs. |
| Alert never fires | See runbook **STOCK-MONITORING-ALERT-DEBUG.md** (quote availability, condition, enabled, already notified). |

---

## 5. Getting symbol lists from Finnhub

When Finnhub is configured (`FINNHUB_API_KEY`), you can get symbol lists with the same token.

### Symbol search (by name or ticker)

Returns best-matching symbols (stocks, ETFs, etc.) for a query:

```bash
# Replace YOUR_FINNHUB_API_KEY with your key (or use the one in remote .env)
curl -sS "https://finnhub.io/api/v1/search?q=AAPL&token=YOUR_FINNHUB_API_KEY"
```

**Response:** `{"count":N,"result":[{"description":"...","displaySymbol":"AAPL","symbol":"AAPL","type":"Common Stock"},...]}`

Use `q` for company name or ticker (e.g. `q=Apple`, `q=ASML`).

### Stock symbols by exchange

Returns all symbols for an exchange (US, EU, etc.):

```bash
curl -sS "https://finnhub.io/api/v1/stock/symbol?exchange=US&token=YOUR_FINNHUB_API_KEY"
# Common exchanges: US, EU, LON, XETRA, AS, etc.
```

**Response:** Array of `{"symbol","displaySymbol","description","type","currency","mic",...}`. For very large lists Finnhub may redirect to a static file URL.

**From the remote host** (reuse the key from the container):

```bash
KEY=$(docker exec lianel-stock-monitoring-service env | grep "^FINNHUB_API_KEY=" | cut -d= -f2-)
curl -sS "https://finnhub.io/api/v1/search?q=ASML&token=${KEY}"
```

See [Finnhub Stock Symbols](https://finnhub.io/docs/api/stock-symbols) and [Symbol Search](https://finnhub.io/docs/api/symbol-search).

---

## Related

- **Backend:** `lianel/dc/stock-monitoring/backend` (symbol validation, quote provider mapping).
- **EU MVP scope:** `lianel/dc/stock-monitoring/docs/EU-MARKETS-MVP.md`.
- **Alert debugging:** `lianel/dc/docs/runbooks/STOCK-MONITORING-ALERT-DEBUG.md`.
