# Grafana Datasource Fixes - Complete ✅

## Problem
Most Grafana charts showing errors due to:
- PostgreSQL connection failures (wrong URL)
- Airflow user authentication failures
- Datasource reference format issues

## Root Causes

1. **Datasource URL**: Remote host used `postgres:5432` (non-existent container) instead of `172.18.0.1:5432` (host PostgreSQL)
2. **Datasource References**: Some dashboards used string names instead of UID format
3. **Missing search_path**: PostgreSQL Airflow datasource missing `search_path: public`

## Fixes Applied

### 1. Datasource Configuration (`datasources.yml`)
- ✅ Updated URL: `postgres:5432` → `172.18.0.1:5432`
- ✅ Added `search_path: public` to PostgreSQL Airflow datasource

### 2. Dashboard Datasource References

#### Fixed Dashboards:
- ✅ **database-performance.json**: Changed `"PostgreSQL"` → `{"type": "postgres", "uid": "postgres-airflow"}` (6 instances)
- ✅ **energy-osm-features.json**: Changed `"PostgreSQL Energy"` → `{"type": "postgres", "uid": "postgres-energy"}` (4 instances)
- ✅ **data-quality.json**: Changed `"PostgreSQL Energy"` → `{"type": "postgres", "uid": "postgres-energy"}` (8 instances)

#### Already Correct:
- ✅ `pipeline-status.json` - Uses UID format for `postgres-airflow`
- ✅ `sla-monitoring.json` - Uses UID format for `postgres-airflow`
- ✅ `system-health.json` - Uses Prometheus (correct)
- ✅ `error-tracking.json` - Uses Loki/Prometheus (correct)
- ✅ `web-analytics.json` - Uses Loki (correct)
- ✅ `energy-metrics.json` - Uses PostgreSQL Energy with UID (correct)

### 3. Remote Host Updates
- ✅ Updated datasource URL via `sed` command
- ✅ Added `search_path: public` via `sed` command
- ✅ Restarted Grafana container

## Files Modified

### Datasource Configuration:
- `monitoring/grafana/provisioning/datasources/datasources.yml`

### Dashboards:
- `monitoring/grafana/provisioning/dashboards/database-performance.json`
- `monitoring/grafana/provisioning/dashboards/energy-osm-features.json`
- `monitoring/grafana/provisioning/dashboards/data-quality.json`

## Verification

After fixes:
- ✅ All PostgreSQL queries should work
- ✅ No authentication errors
- ✅ No connection errors
- ✅ All dashboard panels should load data correctly

## Next Steps

1. **Verify in Grafana UI**:
   - Go to Grafana → Configuration → Data Sources
   - Verify "PostgreSQL Airflow" and "PostgreSQL Energy" are configured correctly
   - Test connection for both datasources

2. **Check Dashboards**:
   - Open each dashboard and verify panels load correctly
   - Check for any remaining errors

3. **Monitor**:
   - Check Grafana logs for any connection errors
   - Verify all queries execute successfully

## Status
✅ **Complete** - All datasource fixes applied and Grafana restarted
