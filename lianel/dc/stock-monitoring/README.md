# Stock Exchange Monitoring and Analysis Service

Stock monitoring and analysis product. **Scope: global markets; MVP focuses on EU markets first.**

## Structure (specific vs shared)

| Area | Location | Description |
|------|----------|-------------|
| **Backend** | `stock-monitoring/backend/` | Rust API service (watchlists, alerts, quotes, EU data). **Specific** to this product. |
| **Frontend** | `stock-monitoring/frontend/` | React UI for dashboards, watchlists, alerts. **Specific** to this product. |
| **Docs** | `stock-monitoring/docs/` | Product documentation, EU MVP scope, API spec. **Specific**. |
| **DAGs** | `lianel/dc/dags/stock_*.py` | Airflow DAGs for ingestion, alerts, reports. **Shared** Airflow; naming prefix `stock_`. |
| **DB migrations** | `lianel/dc/database/migrations/` | Schema for stock_monitoring (e.g. `022_stock_monitoring_*.sql`). **Shared** database repo. |
| **Keycloak** | `lianel/dc/docker-compose.infra.yaml` | Auth and users. **Shared** – add client for stock-monitoring if needed. |
| **Airflow** | `lianel/dc/docker-compose.airflow.yaml` | Scheduler and workers. **Shared** – stock DAGs run here. |
| **Nginx** | `lianel/dc/nginx/config/nginx.conf` | Reverse proxy. **Shared** – add routes for stock-monitoring backend/frontend. |

## MVP scope: EU markets first

- **Exchanges/data:** EU markets (e.g. XETRA, Euronext, LSE, SIX) and EU-listed symbols.
- **Geography:** Expand to North America and other regions after MVP.
- See `docs/EU-MARKETS-MVP.md` and `docs/status/STOCK-EXCHANGE-MONITORING-SERVICE-FUNCTION-LIST.md`.

## Quick start (when implemented)

```bash
# Backend
cd stock-monitoring/backend && cargo run

# Frontend
cd stock-monitoring/frontend && npm install && npm start
```

## Related docs

- **Task checklist:** `stock-monitoring/docs/TASK-CHECKLIST.md` (checkbox list to follow; tick as you complete each task).
- **Implementation plan:** `stock-monitoring/docs/IMPLEMENTATION-PLAN.md` (phases and steps for implementing each part).
- **Architecture design:** `stock-monitoring/docs/ARCHITECTURE-DESIGN.md` (components, deployment, Nginx, Keycloak, Airflow; aligned with Comp-AI and Energy).
- Function list and priorities: `lianel/dc/docs/status/STOCK-EXCHANGE-MONITORING-SERVICE-FUNCTION-LIST.md`
- EU MVP details: `stock-monitoring/docs/EU-MARKETS-MVP.md`
