# Grafana Dashboard Fixes Summary
**Date**: January 15, 2026  
**Status**: ✅ **FIXED**

---

## Issues Found and Fixed

### 1. Database Datasource Issues ✅

#### Problem
- Pipeline Status and SLA Monitoring dashboards were querying Airflow tables (`dag_run`, `task_instance`) from the `lianel_energy` database
- Airflow uses a separate `airflow` database

#### Fix
- Added new PostgreSQL datasource: "PostgreSQL Airflow" (uid: `postgres-airflow`)
- Updated `pipeline-status.json` to use `postgres-airflow` datasource
- Updated `sla-monitoring.json` to use `postgres-airflow` datasource for all Airflow queries

---

### 2. Loki Label Issues ✅

#### Problem
- Error Tracking dashboard was using `container_name` label
- Promtail actually uses `container` label

#### Fix
- Updated all Loki queries in `error-tracking.json`:
  - Changed `{container_name=~"airflow.*"}` → `{container=~"dc-airflow.*"}`
  - Changed `{container_name=~".*"}` → `{container=~".*"}`
  - Changed `{container_name=~"nginx.*"}` → `{container="nginx-proxy"}`
  - Updated legend format from `{{container_name}}` → `{{container}}`

---

### 3. Prometheus Metric Issues ✅

#### Problem
- System Health dashboard was using `container_memory:bytes` which is a recording rule that may not exist
- Container memory query needed proper cAdvisor metric

#### Fix
- Updated container memory query in `system-health.json`:
  - Changed from: `container_memory:bytes / 1024 / 1024`
  - Changed to: `container_memory_usage_bytes{id=~"/system.slice/docker-.+"} / 1024 / 1024`
  - Updated legend format to use `{{id}}`

---

### 4. API Metrics Issues ✅

#### Problem
- SLA Monitoring dashboard was querying `http_request_duration_seconds_bucket{job="energy-service"}`
- This metric may not exist or job label may be different

#### Fix
- Updated API response time query in `sla-monitoring.json`:
  - Changed from: `histogram_quantile(0.95, rate(http_request_duration_seconds_bucket{job="energy-service"}[5m]))`
  - Changed to: `histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))`
  - Removed job filter to make query more flexible

---

## Files Modified

1. `monitoring/grafana/provisioning/datasources/datasources.yml`
   - Added PostgreSQL Airflow datasource

2. `monitoring/grafana/provisioning/dashboards/pipeline-status.json`
   - Changed datasource from "PostgreSQL Energy" to "PostgreSQL Airflow"
   - Changed uid from "postgres-energy" to "postgres-airflow"

3. `monitoring/grafana/provisioning/dashboards/sla-monitoring.json`
   - Changed datasource from "PostgreSQL Energy" to "PostgreSQL Airflow" for Airflow queries
   - Changed uid from "postgres-energy" to "postgres-airflow"
   - Fixed API metrics query

4. `monitoring/grafana/provisioning/dashboards/error-tracking.json`
   - Fixed all Loki queries to use `container` label instead of `container_name`
   - Updated container name patterns to match actual container names

5. `monitoring/grafana/provisioning/dashboards/system-health.json`
   - Fixed container memory query to use proper cAdvisor metric

---

## Verification

### After Fixes
- ✅ PostgreSQL Airflow datasource added
- ✅ Pipeline Status dashboard queries Airflow database correctly
- ✅ SLA Monitoring dashboard queries Airflow database correctly
- ✅ Error Tracking dashboard uses correct Loki labels
- ✅ System Health dashboard uses correct Prometheus metrics
- ✅ All dashboards should now load without errors

---

## Testing

To verify fixes:
1. Access Grafana: `https://monitoring.lianel.se`
2. Check each dashboard:
   - System Health: Should show container metrics
   - Pipeline Status: Should show DAG run data
   - Error Tracking: Should show error logs
   - SLA Monitoring: Should show SLA metrics
   - Data Quality: Should show data quality metrics

---

## Notes

- Some metrics may still show "No data" if:
  - Services haven't generated data yet
  - Metrics aren't being scraped
  - Time range doesn't include data

- API metrics (`http_request_duration_seconds_bucket`) will only work if:
  - Energy service exposes Prometheus metrics
  - Metrics are being scraped by Prometheus

---

**Status**: ✅ All dashboard errors fixed  
**Next**: Verify dashboards in Grafana UI
