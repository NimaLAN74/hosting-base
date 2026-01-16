# Grafana Datasource Fix - Complete ✅
**Date**: January 16, 2026

---

## ✅ Issues Fixed

### Problem 1: Datasource Reference Format
**Issue**: Datasource references using string format instead of object format  
**Fix**: Updated all `"PostgreSQL Airflow"` to `{"type": "postgres", "uid": "postgres-airflow"}`

### Problem 2: Schema Qualification
**Issue**: Queries using unqualified table names (e.g., `dag_run` instead of `public.dag_run`)  
**Fix**: Added explicit schema qualification to all queries:
- `dag_run` → `public.dag_run`
- `task_instance` → `public.task_instance`

### Problem 3: Search Path Configuration
**Issue**: Grafana datasource may not have correct search_path set  
**Fix**: Added `postgresqlOptions: "-c search_path=public"` to datasource configuration

### Problem 4: NULL Handling and Division by Zero
**Issue**: Queries failing on NULL values or division by zero  
**Fix**: Added NULL checks and NULLIF for division operations

### Problem 5: Timestamp Formatting
**Issue**: Time series queries needing explicit timestamp casting  
**Fix**: Added `::timestamp` casting for time columns

---

## 🔧 All Fixes Applied

### Pipeline Status Dashboard
✅ 6 panels fixed with:
- Schema qualification (`public.dag_run`, `public.task_instance`)
- NULL checks for `end_date` and `start_date`
- Division by zero protection
- Timestamp casting for time series

### SLA Monitoring Dashboard
✅ 5 DAG-related panels fixed with:
- Schema qualification (`public.dag_run`)
- NULL checks and NULLIF
- Division by zero protection
- Timestamp casting for time series

### Datasource Configuration
✅ PostgreSQL Airflow datasource updated with:
- Explicit search_path setting
- Correct UID format in queries

---

## 📊 Queries Updated

### Pipeline Status Dashboard
1. **Running DAGs**: `SELECT COUNT(*) FROM public.dag_run WHERE state = 'running';`
2. **Failed DAGs**: `SELECT COUNT(*) FROM public.dag_run WHERE state = 'failed' AND end_date IS NOT NULL ...`
3. **Queued Tasks**: `SELECT COUNT(*) FROM public.task_instance WHERE state = 'queued';`
4. **Success Rate**: `SELECT ... FROM public.dag_run ...`
5. **Execution Duration**: `SELECT end_date::timestamp as time, ... FROM public.dag_run ...`
6. **Status Table**: `SELECT dag_id, state, ... FROM public.dag_run ...`

### SLA Monitoring Dashboard
1. **DAG Success Rate SLA**: `SELECT ... FROM public.dag_run ...`
2. **Avg DAG Duration**: `SELECT AVG(...) FROM public.dag_run ...`
3. **Query Performance**: `SELECT AVG(...) FROM public.dag_run ...`
4. **Success Rate Trend**: `SELECT date_trunc(...)::timestamp as time, ... FROM public.dag_run ...`
5. **Compliance Table**: `SELECT dag_id, ... FROM public.dag_run ...`

---

## ✅ Verification Steps

1. **Check Datasource Connection**:
   - Go to Grafana → Configuration → Data Sources
   - Click "PostgreSQL Airflow"
   - Click "Test" button
   - Should show "Data source is working"

2. **Check Dashboards**:
   - Open "Pipeline Status" dashboard
   - Open "SLA Monitoring" dashboard
   - All panels should load without errors

3. **Verify Queries**:
   - Edit any panel with a DAG query
   - Check query uses `public.dag_run` or `public.task_instance`
   - Verify query has NULL checks and timestamp casting

---

## 🎯 Expected Results

After these fixes:
- ✅ All DAG queries use `public.dag_run` and `public.task_instance`
- ✅ Datasource has explicit search_path set
- ✅ NULL values handled correctly
- ✅ Division by zero prevented
- ✅ Time series queries properly formatted

---

**Status**: ✅ **ALL FIXES DEPLOYED** - All DAG-related queries should now work correctly!
