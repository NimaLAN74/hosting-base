# Stock Monitoring – Debugging Alert Evaluation

**Last updated:** February 2026  
**Owner:** Stock Monitoring / Ops

---

## Overview

Alerts are **price-based**: “above” or “below” a threshold. Evaluation runs:

1. **On demand:** When a user opens the alerts list (`GET /api/v1/alerts`), the backend evaluates that user’s alerts once (best-effort).
2. **Scheduled:** The Airflow DAG `stock_monitoring_alerts` runs after the ingest DAG and calls the backend `POST /internal/alerts/evaluate` to evaluate **all** users’ alerts.

If an alert does not trigger when expected, use this runbook to debug.

---

## 1. How evaluation works

1. Backend loads all **enabled** alerts for the user(s) with `notified_at IS NULL`.
2. Collects unique **symbols** from those alerts and fetches **quotes** from the configured provider (e.g. Yahoo, Alpha Vantage).
3. For each alert: if the current price meets the condition (above/below threshold) and the alert has not already been notified, the backend:
   - Sets `notified_at` on the alert,
   - Inserts a row into `stock_monitoring.notifications`,
   - Writes an audit log entry.

If **no quote** is returned for a symbol, that alert is **skipped** (no trigger, no error). So the most common reason an alert “doesn’t fire” is: **no price data for that symbol** at evaluation time.

---

## 2. Quick checks

| Check | How |
|-------|-----|
| Alert enabled? | `SELECT id, symbol, condition_type, condition_value, enabled, notified_at FROM stock_monitoring.alerts WHERE user_id = '<user_id>';` |
| Already triggered? | `notified_at IS NOT NULL` means it already fired once. |
| Quote available? | Call `GET /api/v1/quotes?symbols=<symbol>` (or check backend logs when evaluation runs). |
| Condition correct? | “above” 100 means trigger when price ≥ 100; “below” 50 means trigger when price ≤ 50. |

---

## 3. Database checks

### Alerts for a user

```bash
# Replace <user_id> with the Keycloak sub (user_id) and set DB connection
psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U $POSTGRES_USER -d $POSTGRES_DB -c "
  SELECT id, symbol, condition_type, condition_value, enabled, notified_at
  FROM stock_monitoring.alerts
  WHERE user_id = '<user_id>'
  ORDER BY id;
"
```

### Notifications (recent triggers)

```bash
psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U $POSTGRES_USER -d $POSTGRES_DB -c "
  SELECT id, user_id, alert_id, message, created_at
  FROM stock_monitoring.notifications
  WHERE user_id = '<user_id>'
  ORDER BY created_at DESC
  LIMIT 20;
"
```

### Audit log (alert triggers)

```bash
psql -h $POSTGRES_HOST -p $POSTGRES_PORT -U $POSTGRES_USER -d $POSTGRES_DB -c "
  SELECT id, user_id, action, resource_type, resource_id, details, created_at
  FROM stock_monitoring.audit_log
  WHERE action = 'alert.trigger'
  ORDER BY created_at DESC
  LIMIT 20;
"
```

---

## 4. Backend logs

Backend uses **tracing**. When evaluation runs:

- **Skipped user (no alerts):** No specific log.
- **Quote fetch failure:** Look for errors from `fetch_provider_quotes` or the HTTP client (e.g. provider timeout, 403, invalid symbol).
- **Alert evaluation failed for user:** `Alert evaluation failed for user <user_id>: <error>` (warning).

Run with `RUST_LOG=info` or `RUST_LOG=lianel_stock_monitoring_service=debug` for more detail.

```bash
docker logs lianel-stock-monitoring-service --tail 200 2>&1 | grep -i alert
```

---

## 5. Airflow DAG

- **DAG id:** `stock_monitoring_alerts`
- **Schedule:** Runs after `stock_monitoring_ingest` (e.g. at :05 past the hour).
- **Task:** `evaluate_stock_alerts` calls `POST <STOCK_MONITORING_INTERNAL_URL>/internal/alerts/evaluate`.

If the DAG fails:

1. **Check internal URL:** `STOCK_MONITORING_INTERNAL_URL` must be reachable from the Airflow worker (e.g. `http://lianel-stock-monitoring-service:3003`).
2. **Check response:** DAG expects HTTP 2xx and a JSON body; non-2xx raises and fails the task.
3. **Check ingest:** Alerts DAG waits for `stock_monitoring_ingest` success. If ingest is placeholder or failing, alerts still run but may have no new quote data (backend fetches on demand from the provider when evaluating).

---

## 6. Common causes “alert didn’t fire”

| Cause | What to do |
|-------|------------|
| **Alert disabled** | Enable in UI or DB: `UPDATE stock_monitoring.alerts SET enabled = true WHERE id = <id>;` |
| **Already triggered** | `notified_at` set; user was already notified. Create a new alert or reset for testing: `UPDATE stock_monitoring.alerts SET notified_at = NULL WHERE id = <id>;` (only for debugging). |
| **No quote for symbol** | Provider may not return that symbol (wrong suffix, unsupported exchange). Check `GET /api/v1/quotes?symbols=<symbol>`. Backend maps e.g. `.ST` → `.STO` for Alpha Vantage. |
| **Condition not met** | Price has not reached threshold yet (above/below). Verify current price and condition_type/condition_value. |
| **Evaluation not run** | For scheduled path: ensure alerts DAG ran and succeeded; check Airflow task logs. For on-demand: user must have loaded the alerts list (or call internal endpoint). |

---

## Related

- **Internal endpoint:** `POST /internal/alerts/evaluate` (no auth; call only from trusted network).
- **Migrations:** `022_stock_monitoring_schema.sql`, `023_stock_monitoring_notifications.sql`.
- **Runbooks:** STOCK-MONITORING-MIGRATIONS.md, STOCK-MONITORING-SYMBOLS.md.
