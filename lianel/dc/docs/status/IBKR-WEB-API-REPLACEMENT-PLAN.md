# IBKR Web API – Complete Replacement Plan for Stock Service

This document is a clear, end-to-end plan to replace the current stock monitoring service (Finnhub/Yahoo/Alpaca + our backend) with an **IBKR Web API–based solution**. It is based on the official [IBKR Web API Documentation](https://ibkrcampus.com/ibkr-api-page/webapi-doc/) and [Client Portal API v1.0](https://interactivebrokers.com/campus/ibkr-api-page/cpapi-v1).

---

## 1. IBKR Web API – Summary

### 1.1 What It Is

- **Single API** unifying Client Portal Web API, Digital Account Management, and Flex Web Service.
- **REST + WebSocket**: HTTP JSON for most operations; WebSocket for streaming market data.
- **Base URL (OAuth)**: `https://api.ibkr.com/v1/api`  
  (Client Portal Gateway: `https://localhost:5000/v1/api` – local only, not for our server.)

### 1.2 Authentication Options

| Method | Use case | Server-side suitable? |
|--------|----------|------------------------|
| **OAuth 2.0 (beta)** | Account management + trading | Yes – our backend can call api.ibkr.com with user tokens |
| **OAuth 1.0a** | Trading only | Yes – same |
| **SSO** | FAs/IBs, alternative UIs | No – for advisor/broker setups |
| **Client Portal Gateway (CPGW)** | Individual accounts, local Java app | Optional – we can run it in Docker and point the backend at it when api.ibkr.com returns “no bridge”; see stock-service/docs/CLIENT-PORTAL-GATEWAY.md |

For a **server-side replacement**, we must use **OAuth 1.0a or OAuth 2.0** so our backend can call `https://api.ibkr.com/v1/api` on behalf of the user. If market data calls return “no bridge”, you can optionally deploy the **Client Portal Gateway** (Docker) and set `IBKR_API_BASE_URL=https://ibkr-gateway:5000/v1/api` (see stock-service docs). Each app user must **link their IBKR account** (OAuth flow); we store and refresh tokens and call IBKR from our service.

### 1.3 Account & Market Data Requirements

- **IBKR Pro**, **fully open and funded** account.
- **Market data subscriptions** enabled for the instruments you want (e.g. US equities, EU). Market data is **not free**; user must subscribe in IBKR account.
- **Session**: A “brokerage session” is required for `/iserver/*` (trading, market data). Session stays alive with periodic `/tickle` (e.g. every ~1 min). Session can last up to ~24h; resets at regional midnight (NA/Europe/Asia).
- **Cookie**: After OAuth, client must call `/tickle`, get session token, and send cookie `api={session_token}` on subsequent requests.

### 1.4 Rate Limits (relevant subset)

| Endpoint | Limit |
|----------|--------|
| Global (per session) | 10 req/s |
| `/iserver/marketdata/snapshot` | 10 req/s |
| `/iserver/marketdata/history` | 5 **concurrent** requests |
| `/iserver/scanner/params` | 1 req / 15 min |
| `/iserver/scanner/run` | 1 req/s |
| `/iserver/account/orders`, `/iserver/account/trades` | 1 req / 5 s |
| `/portfolio/accounts` | 1 req / 5 s |
| `/tickle` | 1 req/s |

On violation: **429 Too Many Requests**; IP can be penalized (e.g. 10–15 min block). Repeat violations can lead to permanent block.

### 1.5 Maintenance

- **Web API**: Maintenance Saturday evenings.
- **Brokerage (`/iserver`)**: Briefly unavailable ~01:00 local time by region (NA, Europe, Asia).

---

## 2. Key Endpoints for Our Use Case

### 2.1 Symbol → Contract ID (conid)

- **`GET /trsrv/stocks?symbols=AAPL,MSFT`**  
  Returns conids and metadata per symbol. **Does not require brokerage session.**  
  Use this for **watchlist symbols**: resolve symbol → conid once, store conid (and optionally exchange) in our DB.

- **`GET /iserver/secdef/search?symbol=AAPL&name=true`**  
  Search by company name or ticker. **Requires brokerage session.** Use for **symbol search in UI** (by name).  
  Response: `conid`, `companyName`, `symbol`, `sections` (exchanges, etc.).

- **`GET /trsrv/secdef?conids=265598`**  
  Full contract details for given conids. No session required. Optional for display (e.g. currency, listing exchange).

### 2.2 Live Quotes (Top-of-Book)

- **`GET /iserver/marketdata/snapshot?conids=265598,8314&fields=31,84,86`**  
  - **31** = Last price, **84** = Bid, **86** = Ask (see [Market Data Fields](https://ibkrcampus.com/ibkr-api-page/webapi-ref/#tag/Market-Data)).  
  - **Pre-flight**: First request for a conid opens the stream; response may return only conids. Subsequent requests return data.  
  - Max **100 conids**, **50 fields** per request.  
  - **Requires**: brokerage session + market data subscription for those instruments.

### 2.3 Historical Data

- **`GET /iserver/marketdata/history?conid=265598&exchange=SMART&period=7d&bar=1d`**  
  - **period**: `1min`–`30min`, `1h`–`8h`, `1d`–`1000d`, `1w`–`792w`, `1m`–`182m`, `1y`–`15y`.  
  - **bar**: `1min`, `2min`, … `1h`, `1d`, `1w`, `1m`.  
  - **Step size**: Period and bar must match [documented combinations](https://interactivebrokers.com/campus/ibkr-api-page/cpapi-v1) (e.g. 7d → bar 1d; 1d → bar 1min).  
  - **Limit**: 5 **concurrent** history requests per session.  
  - Use for: **7-day chart** (`period=7d`, `bar=1d`) and **intraday chart** (`period=1d`, `bar=1min`).

### 2.4 Alerts (IBKR Native)

- **`POST /iserver/account/{accountId}/alert`**  
  Create/modify alert. Body: `alertName`, `alertMessage`, `tif` (GTC/GTD), `conditions` (e.g. type 1 = Price, `conidex`, `operator`, `value`), `expireTime` (if GTD), etc.  
  Alerts fire in TWS, IBKR app, or email.  
  We can **replace our DB-backed alerts** with IBKR alerts, or keep our alerts and **mirror** them to IBKR for execution.

### 2.5 Portfolio & Account

- **`GET /portfolio/accounts`** – List accounts (call before other `/portfolio` calls).  
- **`GET /iserver/accounts`** – Accounts the user can trade (needed before snapshot).  
- **`GET /portfolio/{accountId}/positions`** – Positions (optional for “my holdings” view).

### 2.6 Session & Tickle

- **`GET /tickle`** – Keep session alive; response includes session token for cookie `api={token}`.  
- **`GET /iserver/auth/status`** – `authenticated`, `connected`, `competing`, `established`.  
- **`POST /logout`** – Invalidate session; client must discard cookies.

---

## 3. Current Stock Service vs IBKR Mapping

### 3.1 Current Backend Surface (to preserve or adapt)

| Current | Purpose | IBKR-based approach |
|--------|---------|----------------------|
| `GET /api/v1/me` | Current user | Keep; add optional `ibkr_account_linked`, `ibkr_account_id`. |
| `GET/POST/PUT/DELETE /api/v1/watchlists` | Watchlists CRUD | Keep our DB; **store conid + exchange** per item (from `/trsrv/stocks` or `/iserver/secdef/search`). |
| `GET/POST/DELETE /api/v1/watchlists/:id/items` | Watchlist items | Same; add item fields: `conid`, `exchange` (e.g. SMART). Symbol stays for display. |
| `GET /api/v1/quotes?pairs=S:P,...` | Latest prices | Backend calls **IBKR snapshot** for stored conids; map back to symbol. Cache and respect 10 req/s. |
| `GET /api/v1/price-history/daily?symbol=&provider=&days=7` | 7-day OHLC | Backend calls **IBKR history** `period=7d&bar=1d` for symbol’s conid. Optional: persist to our DB for offline/cache. |
| `GET /api/v1/price-history/intraday?symbol=&provider=` | Today’s ticks | Backend calls **IBKR history** `period=1d&bar=1min` for conid. Optional: cache in Redis/DB. |
| `GET /api/v1/symbols/providers` | Provider list | Replace with single “IBKR” or remove; symbol search uses IBKR. |
| `GET /api/v1/symbols?q=` | Symbol search | Backend calls **`/iserver/secdef/search?symbol=...&name=true`**; return list with conid, name, symbol. |
| `GET/POST/PUT/DELETE /api/v1/alerts` | Alerts CRUD | Option A: Keep our DB and **mirror** to IBKR `POST /iserver/account/{accountId}/alert`. Option B: Use only IBKR alerts and proxy list/create/delete via IBKR (if list/delete endpoints exist). |
| `GET /api/v1/notifications` | Notifications | Proxy **`GET /fyi/notifications`** (and mark-read via IBKR). |
| Internal: ingest, roll-daily, webhooks | Ingest from providers | **Remove** Finnhub/Yahoo/Alpaca ingest; no webhooks. Optional: scheduled job that calls IBKR history for watchlist conids and fills our DB for 7d cache. |

### 3.2 What We Remove

- All **Finnhub, Yahoo, Alpaca** (and similar) provider code and env vars.
- **Provider** as a first-class concept in the API: single source of truth is IBKR. We can keep `provider` column as `'ibkr'` for backward compatibility or drop it.
- **Internal ingest DAG** that called multiple providers; replace with optional “IBKR history sync” job if we persist history in our DB.
- **Finnhub webhooks** and any other provider webhooks.

### 3.3 What We Keep (and adapt)

- **Keycloak** for **app** authentication (unchanged).  
- **Our DB**: watchlists, watchlist_items (add conid, exchange); alerts (either keep and mirror to IBKR or migrate to IBKR-only).  
- **Frontend** routing and structure; only API contract and “provider” UX change (single data source: IBKR).

---

## 4. Architecture – Backend as IBKR Proxy

### 4.1 Flow

1. **App user** logs in with **Keycloak** (unchanged).  
2. **Linking IBKR**: User goes to “Connect IBKR” (or similar); we run **OAuth 1.0a or 2.0** with IBKR, get tokens, store encrypted in our DB (e.g. `user_ibkr_credentials` keyed by `user_id`).  
3. **Session**: When we need to call `/iserver/*`, we ensure an active brokerage session:  
   - Use stored refresh token / credentials to obtain session (OAuth flow + `/tickle`).  
   - Store session cookie (or equivalent) per user/session; call **`/tickle`** periodically (e.g. every 60 s) while user is active.  
4. **Requests**: Frontend continues to call **our** backend at `/api/v1/stock-service/*`. Our backend:  
   - Resolves symbol → conid (from our DB or `/trsrv/stocks` / `/iserver/secdef/search`).  
   - Calls IBKR `api.ibkr.com` with user’s session cookie / token.  
   - Maps IBKR responses to existing response shapes where possible (e.g. quotes, history).  
5. **Rate limiting**: We must respect IBKR’s 10 req/s and per-endpoint limits; implement client-side throttling and optional response caching.

### 4.2 Where to Run IBKR Calls

- **Backend only**: All IBKR calls from our **stock-service backend** (Rust). No IBKR credentials or session in the browser.  
- **Session per user**: One IBKR session per linked user (or per account). Session lifecycle: create on first need, keep alive with `/tickle`, invalidate on logout or expiry.

### 4.3 Conid Resolution and Storage

- **Watchlist add**: When user adds “AAPL”, we call `/trsrv/stocks?symbols=AAPL` (or secdef/search if by name). We get conid + exchange. We store in `watchlist_items`: `symbol`, `conid`, `exchange` (and drop or fix `provider` to `'ibkr'`).  
- **Quotes**: We have conids in DB; we batch snapshot (up to 100 per request) and return by symbol.  
- **History**: We have conid; we call history once per symbol (respect 5 concurrent).

---

## 5. Data Model Changes

### 5.1 watchlist_items

- **Add**: `conid BIGINT`, `exchange VARCHAR(32)` (e.g. SMART).  
- **Keep**: `symbol` (display).  
- **Provider**: Set to `'ibkr'` for all new rows; optionally migrate existing rows to conid by resolving symbol via `/trsrv/stocks` (or leave legacy provider data read-only until migration).

### 5.2 New table: user_ibkr_credentials (or equivalent)

- `user_id` (our Keycloak user).  
- Encrypted **OAuth tokens** (access, refresh).  
- Optional: `ibkr_account_id` (for which account we use for alerts/snapshot).  
- `updated_at`, `linked_at`.

### 5.3 Alerts

- **Option A**: Keep `stock_service.alerts`; add `ibkr_alert_id` if we mirror to IBKR. On create/update, call `POST /iserver/account/{accountId}/alert`.  
- **Option B**: Remove our alerts table; use only IBKR alerts and implement list/create/delete via IBKR (need to confirm list/delete API).

### 5.4 Price history tables (optional)

- If we **stop** persisting daily/intraday in our DB, we can leave tables as-is (or drop later).  
- If we **keep** a cache: we can have a job that calls IBKR history for watchlist conids and writes to `price_history_daily` / `price_history_intraday` with `provider='ibkr'`.

---

## 6. Implementation Phases

### Phase 1: Prerequisites and OAuth Integration

- [ ] **IBKR developer setup**: Register app for OAuth 1.0a (or 2.0 beta); get consumer key/secret; configure callback URL.  
- [ ] **Backend**: Add module for IBKR OAuth flow (authorize URL, callback handler, token exchange).  
- [ ] **DB**: Add `user_ibkr_credentials` (or equivalent) and migrations.  
- [ ] **Frontend**: “Connect IBKR account” page; redirect to IBKR; callback stores tokens and shows “Linked”.  
- [ ] **Session**: Implement “get or create IBKR session” (OAuth + `/tickle`), store cookie/token for server-side use; background tickle every ~60 s when user is active.

### Phase 2: Symbol Resolution and Watchlist

- [ ] **Backend**:  
  - `GET /trsrv/stocks?symbols=...` (no session) for bulk symbol → conid.  
  - `GET /iserver/secdef/search?symbol=...&name=true` for search-by-name (with session).  
- [ ] **DB**: Migration adding `conid`, `exchange` to `watchlist_items`; backfill conids for existing symbols via `/trsrv/stocks` where possible.  
- [ ] **API**: When user adds watchlist item, resolve conid (and exchange); save symbol + conid + exchange.  
- [ ] **Frontend**: Symbol search calls our backend; we proxy to IBKR secdef/search; display results with symbol/name; on add, we send symbol (backend resolves conid and stores).

### Phase 3: Quotes (Snapshot)

- [ ] **Backend**:  
  - Implement snapshot call: `GET /iserver/marketdata/snapshot?conids=...&fields=31,84,86`.  
  - Pre-flight once per conid (or batch); then subsequent requests return last/bid/ask.  
  - Map to current quotes response shape (symbol, price, currency, etc.) using stored symbol from watchlist.  
- [ ] **Caching**: Optional in-memory or Redis cache with short TTL (e.g. 60 s) to stay under 10 req/s when many users share same symbols.  
- [ ] **API**: Keep `GET /api/v1/quotes` (or equivalent) taking list of symbols; backend uses stored conids and returns prices.  
- [ ] **Frontend**: No change to “latest prices” UX; only data source is IBKR.

### Phase 4: Historical Data (7-Day and Intraday)

- [ ] **Backend**:  
  - `GET /iserver/marketdata/history?conid=...&period=7d&bar=1d` for 7-day.  
  - `GET /iserver/marketdata/history?conid=...&period=1d&bar=1min` for intraday.  
  - Enforce 5 concurrent history requests (queue or semaphore).  
- [ ] **API**:  
  - `GET /api/v1/price-history/daily?symbol=...&days=7` → backend resolves symbol to conid, calls history, returns OHLC (map IBKR format to our format).  
  - `GET /api/v1/price-history/intraday?symbol=...` → same with period=1d, bar=1min.  
- [ ] **Optional**: Persist to our DB for caching/offline; then we can keep same internal endpoints and roll logic, with “source = IBKR”.

### Phase 5: Alerts and Notifications

- [ ] **Alerts**:  
  - If mirroring: on create/update alert in our DB, call `POST /iserver/account/{accountId}/alert` with conid (from watchlist or resolution), condition (e.g. price), value. Store `ibkr_alert_id` if returned.  
  - If IBKR-only: implement list (if IBKR provides it) and create/delete via API.  
- [ ] **Notifications**: Proxy `GET /fyi/notifications` and mark-read; map to our notifications response if we keep the same UX.

### Phase 6: Decommission Old Providers and Cleanup

- [ ] Remove Finnhub, Yahoo, Alpaca (and any other) provider code, env vars, and webhooks.  
- [ ] Remove or simplify “provider” in API (e.g. always `ibkr` or omit).  
- [ ] Remove internal ingest DAG that called multiple providers; replace with optional IBKR history sync job if we keep DB cache.  
- [ ] Update docs, runbooks, and deployment configs.  
- [ ] Retire `price_history_*` tables or keep only for IBKR-sourced cache.

---

## 7. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| IBKR account not funded / no market data | Show clear message in UI: “Link an IBKR Pro account with market data subscriptions.” |
| Rate limits (429) | Throttle in backend (10 req/s per user/session); cache snapshot and history where appropriate. |
| Session expiry / competing session | Detect via `/iserver/auth/status`; prompt user to re-auth or close other TWS/Portal sessions. |
| OAuth complexity (1.0a vs 2.0) | Start with the one IBKR approves for our use case; document callback and token storage securely. |
| History 5 concurrent | Queue history requests (e.g. 5 workers); don’t fire 50 at once. |
| Maintenance window | Avoid critical path during 01:00 regional; show “Market data temporarily unavailable” if needed. |

---

## 8. Documentation References

- [Web API Documentation (overview)](https://ibkrcampus.com/ibkr-api-page/webapi-doc/)  
- [Client Portal API v1.0 (detailed endpoints)](https://interactivebrokers.com/campus/ibkr-api-page/cpapi-v1)  
- [Web API Reference (Swagger)](https://ibkrcampus.com/ibkr-api-page/webapi-ref/) – for exact request/response schemas.  
- Market Data Fields: field IDs (e.g. 31 = Last, 84 = Bid, 86 = Ask) in Reference.  
- [Market Data Subscriptions](https://www.interactivebrokers.com/campus/ibkr-api-page/market-data-subscriptions/) – user must subscribe.

---

## 9. Summary Checklist

- [ ] Use **OAuth 1.0a or 2.0** so our backend can call `https://api.ibkr.com/v1/api`.  
- [ ] **Link** each app user to one IBKR account (tokens in DB); maintain **brokerage session** and **tickle** periodically.  
- [ ] **Symbol → conid**: `/trsrv/stocks` or `/iserver/secdef/search`; store **conid + exchange** in watchlist_items.  
- [ ] **Quotes**: `/iserver/marketdata/snapshot` (pre-flight + fields 31,84,86); respect 10 req/s.  
- [ ] **7-day / intraday**: `/iserver/marketdata/history` with period/bar; respect 5 concurrent.  
- [ ] **Alerts**: Use IBKR `POST /iserver/account/{accountId}/alert` (and optional mirror in our DB).  
- [ ] **Notifications**: Proxy `/fyi/notifications`.  
- [ ] Remove all **other providers** and provider-specific ingest; optional DB cache for history from IBKR only.  
- [ ] **Rate limiting and caching** in our backend to avoid 429 and improve UX.

This plan is the basis for replacing the stock service completely with an IBKR-based solution while keeping the same app auth (Keycloak) and a compatible API surface for the frontend.

---

## Appendix: IBKR Endpoint Quick Reference

| Purpose | Method | Endpoint | Session |
|--------|--------|----------|---------|
| Symbol → conid (bulk) | GET | `/trsrv/stocks?symbols=AAPL,MSFT` | No |
| Symbol/name search | GET | `/iserver/secdef/search?symbol=...&name=true` | Yes |
| Contract details | GET | `/trsrv/secdef?conids=265598` | No |
| Live quote (last/bid/ask) | GET | `/iserver/marketdata/snapshot?conids=...&fields=31,84,86` | Yes |
| Historical (7d daily bars) | GET | `/iserver/marketdata/history?conid=...&period=7d&bar=1d&exchange=SMART` | Yes |
| Historical (1d minute bars) | GET | `/iserver/marketdata/history?conid=...&period=1d&bar=1min&exchange=SMART` | Yes |
| Create alert | POST | `/iserver/account/{accountId}/alert` | Yes |
| Keep session alive | GET | `/tickle` | - |
| Auth status | GET | `/iserver/auth/status` | - |
| Accounts (trading) | GET | `/iserver/accounts` | Yes |
| Portfolio accounts | GET | `/portfolio/accounts` | No (read-only) |
| Notifications | GET | `/fyi/notifications` | Yes |

**Market data field IDs (snapshot):** 31 = Last, 84 = Bid, 86 = Ask, 85 = Ask size, 88 = Bid size, 7059 = Last size.
