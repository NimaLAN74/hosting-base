# Grafana DAG Dashboard Fixes
**Date**: January 16, 2026

---

## ✅ Issue Fixed

**Problem**: All DAG-related charts in Grafana dashboards were returning errors.

**Root Cause**: Datasource references were using the old string format `"PostgreSQL Airflow"` instead of the object format with UID.

---

## 🔧 Fixes Applied

### 1. Pipeline Status Dashboard
**File**: `monitoring/grafana/provisioning/dashboards/pipeline-status.json`

**Fixed Panels**:
- Running DAGs
- Failed DAGs (24h)
- Queued Tasks
- DAG Success Rate (24h)
- DAG Execution Duration (24h)
- DAG Run Status by DAG (24h)

**Change**: Updated all 6 datasource references from:
```json
"datasource": "PostgreSQL Airflow"
```

To:
```json
"datasource": {
  "type": "postgres",
  "uid": "postgres-airflow"
}
```

---

### 2. SLA Monitoring Dashboard
**File**: `monitoring/grafana/provisioning/dashboards/sla-monitoring.json`

**Fixed Panels**:
- DAG Success Rate SLA
- Avg DAG Duration
- Data Freshness
- Query Performance
- DAG Success Rate Trend (7 days)
- SLA Compliance by DAG (7 days)

**Change**: Updated all 6 datasource references to use the correct UID format.

---

## ✅ Verification

All SQL queries tested and working:
- ✅ `dag_run` table accessible
- ✅ `task_instance` table accessible
- ✅ Queries return correct results
- ✅ Time series queries format correctly

---

## 📊 Dashboard Status

### Pipeline Status Dashboard
**URL**: `https://monitoring.lianel.se/d/pipeline-status`

**Panels** (all fixed):
1. Running DAGs - Count of currently running DAGs
2. Failed DAGs (24h) - Count of failed DAGs in last 24 hours
3. Queued Tasks - Count of queued tasks
4. DAG Success Rate (24h) - Percentage of successful DAGs
5. DAG Execution Duration (24h) - Time series of execution times
6. DAG Run Status by DAG (24h) - Table of DAG statuses

### SLA Monitoring Dashboard
**URL**: `https://monitoring.lianel.se/d/sla-monitoring`

**DAG-Related Panels** (all fixed):
1. DAG Success Rate SLA - 7-day success rate
2. Avg DAG Duration - Average execution time
3. DAG Success Rate Trend (7 days) - Time series
4. SLA Compliance by DAG (7 days) - Table with metrics

---

## 🎯 Next Steps

1. ✅ **Fixed** - Datasource references updated
2. ✅ **Deployed** - Grafana restarted with new configuration
3. ⏳ **Verify** - Check dashboards in Grafana UI to confirm all panels load

---

**Status**: ✅ **FIXED** - All DAG-related charts should now work correctly!
