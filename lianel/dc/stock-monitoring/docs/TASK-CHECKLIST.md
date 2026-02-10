# Stock Monitoring – Task Checklist

**Use this as your day-to-day checklist.** Tick each box when the task is done. For full detail (what, where, how, done when) see [IMPLEMENTATION-PLAN.md](./IMPLEMENTATION-PLAN.md).

**Scope:** EU markets MVP (P0).

---

## Phase 1 – Foundation

- [ ] **1.1** Apply DB migration 022 — Run `022_stock_monitoring_schema.sql` on shared PostgreSQL; confirm schema and tables exist.
- [ ] **1.2** Backend: config and DB pool — Add env config and PgPool; wire into app state.
- [x] **1.3** Backend: Keycloak JWT validation — Validate Bearer token (JWKS); extract `user_id` (sub); 401 when invalid.
- [x] **1.4** Backend: route groups — Public: `/health`, `/api/v1/status`. Protected: `/api/v1/me` (and future routes) behind auth.
- [x] **1.5** Docker Compose — Add `docker-compose.stock-monitoring.yaml`; service on port 3003, healthcheck, `lianel-network`.
- [x] **1.6** Nginx — Add `/api/v1/stock-monitoring/health` (public) and `/api/v1/stock-monitoring/` (HTTPS, auth_request, rewrite to backend).
- [x] **1.7** Keycloak — Doc in `docs/KEYCLOAK-CLIENT.md`; reuse existing client or create one for frontend; backend uses JWKS only.

**Phase 1 done when:** Backend runs in Docker, Nginx proxies to it, DB schema applied, protected route returns 401/200 with token.

---

## Phase 2 – Watchlists and audit (P0.2, P0.7)

- [ ] **2.1** Backend: watchlist models and DB — Structs and queries for `watchlists` and `watchlist_items`; enforce `user_id`.
- [ ] **2.2** Backend: watchlist REST API — GET/POST watchlists; GET/POST/DELETE items; all scoped by user.
- [ ] **2.3** Backend: audit logging — Helper to insert into `audit_log`; call on every watchlist/item create/update/delete.
- [ ] **2.4** Backend: symbol validation — Validate symbol format and optional EU MIC when adding to watchlist; 400 on invalid.

**Phase 2 done when:** Watchlist CRUD works via API; every mutation is in `audit_log`; EU symbols validated.

---

## Phase 3 – Quotes and data (P0.1)

- [ ] **3.1** Quote storage decision — Document: DB table vs on-demand provider vs hybrid; decide and document.
- [ ] **3.2** Migration 023 quote cache — If using DB: add `stock_monitoring.quote_cache` (symbol, price, updated_at, etc.).
- [ ] **3.3** Airflow: ingest DAG — Implement `stock_monitoring_ingest_dag`: fetch EU symbols from provider, write to DB (or backend).
- [ ] **3.4** Backend: quotes API — `GET /api/v1/quotes?symbols=...` returns latest price and last update for requested symbols.
- [ ] **3.5** Backend: last update — Expose last ingestion time (e.g. in status or quotes response) for dashboard “Data as of”.

**Phase 3 done when:** Quotes are ingested on schedule; quotes API works; dashboard can show last update.

---

## Phase 4 – Alerts (P0.3)

- [ ] **4.1** Backend: alert model and DB — Queries for alerts (list by user, get, create, update, delete, list by symbol).
- [ ] **4.2** Backend: alerts REST API — GET/POST/PUT/DELETE alerts; enable/disable; audit on changes.
- [ ] **4.3** Backend: alert evaluation — Logic to check price vs conditions; set `notified_at`; write audit_log; prepare for notification.
- [ ] **4.4** Airflow: alert evaluation DAG — Optional DAG that runs after ingest and triggers backend evaluation (or backend runs it).

**Phase 4 done when:** Alert CRUD works; price conditions are evaluated; triggered alerts recorded for notifications.

---

## Phase 5 – Frontend (P0.5, P0.6)

- [ ] **5.1** Frontend: auth and API client — Keycloak login; Bearer token; API client with base URL `/api/v1/stock-monitoring`.
- [ ] **5.2** Frontend: dashboard — One screen: watchlist symbols, latest price, last update time, simple up/down trend.
- [ ] **5.3** Frontend: watchlist management — Create/rename/delete watchlists; add/remove symbols (with validation feedback).
- [ ] **5.4** Frontend: alert management — Create/edit/delete alerts; set condition type and value; list with enabled/triggered state.
- [ ] **5.5** Frontend: routing and layout — Routes for Dashboard, Watchlists, Alerts; nav; header with user and logout.
- [ ] **5.6** Nginx: serve frontend — Build React app; serve at `/stock` (or configured path); SPA fallback.

**Phase 5 done when:** Dashboard shows watchlist quotes; watchlist and alert CRUD in UI; Keycloak login end-to-end.

---

## Phase 6 – Notifications (P0.4)

- [ ] **6.1** Notifications: one channel — Implement either email (on trigger) or in-app (notifications table + GET API + UI list).
- [ ] **6.2** Wire trigger to notification — From alert evaluation, on trigger: send email or insert in-app notification.

**Phase 6 done when:** When an alert triggers, user receives notification via chosen channel.

---

## Phase 7 – Polish

- [ ] **7.1** OpenAPI and Swagger UI — Document all endpoints; serve Swagger at `/swagger-ui` or `/api-doc`.
- [ ] **7.2** Backend tests — Unit and integration tests for main flows (watchlist, alerts, quotes); run in CI.
- [ ] **7.3** Runbooks and docs — How to run migrations, add symbols, debug alerts; update README/architecture as built.
- [ ] **7.4** (Optional) Alert hub — List all alerts with filter/search, recent triggers, bulk enable/disable (P1.7).

**Phase 7 done when:** API documented; tests pass; runbooks available; optional alert hub if done.

---

## Quick reference

| Phase | Focus              | # tasks |
|-------|--------------------|--------|
| 1     | Foundation         | 7      |
| 2     | Watchlists + audit | 4      |
| 3     | Quotes & data      | 5      |
| 4     | Alerts             | 4      |
| 5     | Frontend           | 6      |
| 6     | Notifications      | 2      |
| 7     | Polish             | 4      |
| **Total** |                   | **32** |

**Detail:** [IMPLEMENTATION-PLAN.md](./IMPLEMENTATION-PLAN.md)  
**Architecture:** [ARCHITECTURE-DESIGN.md](./ARCHITECTURE-DESIGN.md)  
**EU scope:** [EU-MARKETS-MVP.md](./EU-MARKETS-MVP.md)
