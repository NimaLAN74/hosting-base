# Stock Monitoring Frontend

React UI for stock exchange monitoring and analysis. **MVP: EU markets.**

## Shared vs specific

- **Auth:** Keycloak (shared) – reuse existing realm and client or add `stock-monitoring` client.
- **API:** Calls `stock-monitoring/backend` (this product). No dependency on comp-ai or energy frontends.
- **Deploy:** Can be served under same host (e.g. `/stock` or subdomain) or standalone; nginx routes TBD.

## Run locally

```bash
npm install
npm start
```

## Planned (from function list)

- Dashboard (watchlist prices, last update)
- Watchlist CRUD
- Alert creation and management
- Notifications (email/in-app)
