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

## 3. Ingest DAG – symbol list (when implemented)

The **ingest DAG** (`stock_monitoring_ingest`) is currently a **placeholder**: it does not yet fetch real quotes or use a configurable symbol list.

When implemented:

- **Where to configure symbols:** In the DAG code or in Airflow Variables (e.g. a list of EU symbols or symbol+MIC pairs). Alternatively, the DAG could query the backend or DB for “all symbols in watchlists and alerts” to drive ingestion.
- **Provider format:** The backend quote service supports provider-specific mapping (e.g. `.ST` → `.STO` for Alpha Vantage). The ingest pipeline should use the same symbol format as the backend so that alert evaluation sees consistent symbols.
- **Schedule:** Ingest runs on a schedule (e.g. hourly 08–17 UTC weekdays); the alerts DAG runs shortly after to evaluate alerts.

Until the ingest DAG is implemented, **quotes are fetched on demand** by the backend from the configured data provider (e.g. Yahoo or Alpha Vantage) when users open the dashboard or when the alert evaluation job runs. No separate “add symbols for ingest” step is required for MVP.

---

## 4. Troubleshooting

| Issue | Check |
|------|--------|
| Symbol rejected (400) | Ensure format is valid (alphanumeric + optional `.MIC`). See backend validation in `app.rs` (symbol normalization). |
| No quote for symbol | Provider may not support that exchange or symbol; check backend logs and provider docs. |
| Alert never fires | See runbook **STOCK-MONITORING-ALERT-DEBUG.md** (quote availability, condition, enabled, already notified). |

---

## Related

- **Backend:** `lianel/dc/stock-monitoring/backend` (symbol validation, quote provider mapping).
- **EU MVP scope:** `lianel/dc/stock-monitoring/docs/EU-MARKETS-MVP.md`.
- **Alert debugging:** `lianel/dc/docs/runbooks/STOCK-MONITORING-ALERT-DEBUG.md`.
