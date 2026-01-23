# Data Quality Dashboard Column Fix

## Problem
The Data Quality Monitoring dashboard had missing columns in some charts because the queries were using incorrect column names.

## Root Cause
The dashboard queries were using:
- `ingestion_date` instead of `ingestion_timestamp`
- `source_name` instead of `source_system`
- Missing `table_name` column in the "Data Source Status" table

## Actual Schema
From `meta_ingestion_log` table:
- `source_system` (VARCHAR(50)) - NOT `source_name`
- `table_name` (VARCHAR(100))
- `ingestion_timestamp` (TIMESTAMP) - NOT `ingestion_date`
- `records_inserted`, `records_updated`, `records_failed`
- `status`, `error_message`, etc.

## Fixes Applied

### 1. Data Freshness Panel
**Before:**
```sql
SELECT EXTRACT(EPOCH FROM (NOW() - MAX(ingestion_date))) as data_age 
FROM meta_ingestion_log 
WHERE ingestion_date IS NOT NULL;
```

**After:**
```sql
SELECT EXTRACT(EPOCH FROM (NOW() - MAX(ingestion_timestamp))) as data_age 
FROM meta_ingestion_log 
WHERE ingestion_timestamp IS NOT NULL;
```

### 2. Data Freshness Trend (7 days)
**Before:**
```sql
SELECT date_trunc('hour', ingestion_date) as time, 
       EXTRACT(EPOCH FROM (NOW() - MAX(ingestion_date))) as data_age 
FROM meta_ingestion_log 
WHERE ingestion_date > NOW() - INTERVAL '7 days' 
GROUP BY time ORDER BY time;
```

**After:**
```sql
SELECT date_trunc('hour', ingestion_timestamp) as time, 
       EXTRACT(EPOCH FROM (NOW() - MAX(ingestion_timestamp))) as data_age 
FROM meta_ingestion_log 
WHERE ingestion_timestamp > NOW() - INTERVAL '7 days' 
GROUP BY time ORDER BY time;
```

### 3. Ingestion Frequency (30 days)
**Before:**
```sql
SELECT date_trunc('day', ingestion_date) as time, COUNT(*) as ingestion_count 
FROM meta_ingestion_log 
WHERE ingestion_date > NOW() - INTERVAL '30 days' 
GROUP BY time ORDER BY time;
```

**After:**
```sql
SELECT date_trunc('day', ingestion_timestamp) as time, COUNT(*) as ingestion_count 
FROM meta_ingestion_log 
WHERE ingestion_timestamp > NOW() - INTERVAL '30 days' 
GROUP BY time ORDER BY time;
```

### 4. Data Source Status (30 days) - Added table_name column
**Before:**
```sql
SELECT source_name, COUNT(*) as record_count, 
       MAX(ingestion_date) as last_ingestion, 
       EXTRACT(EPOCH FROM (NOW() - MAX(ingestion_date))) as age_seconds 
FROM meta_ingestion_log 
WHERE ingestion_date > NOW() - INTERVAL '30 days' 
GROUP BY source_name 
ORDER BY age_seconds DESC;
```

**After:**
```sql
SELECT source_system, table_name, COUNT(*) as record_count, 
       MAX(ingestion_timestamp) as last_ingestion, 
       EXTRACT(EPOCH FROM (NOW() - MAX(ingestion_timestamp))) as age_seconds 
FROM meta_ingestion_log 
WHERE ingestion_timestamp > NOW() - INTERVAL '30 days' 
GROUP BY source_system, table_name 
ORDER BY age_seconds DESC;
```

## Files Modified
- `lianel/dc/monitoring/grafana/provisioning/dashboards/data-quality.json`

## Status
✅ **Fixed** - All queries now use correct column names and include table_name in the status table

## Next Steps
1. Restart Grafana to reload the dashboard:
   ```bash
   docker-compose -f docker-compose.monitoring.yaml restart grafana
   ```
2. Refresh the Data Quality dashboard in Grafana
3. Verify all charts now display data correctly
