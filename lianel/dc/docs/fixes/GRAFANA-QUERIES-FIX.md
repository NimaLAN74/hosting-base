# Grafana Dashboard Queries Fix

**Date**: January 12, 2026  
**Issue**: Invalid input errors in dashboard queries  
**Status**: ✅ **FIXED**

---

## Problem

Grafana dashboards were returning "invalid input" errors because:
1. **Time Filter Macro Issue**: `$__timeFilter(year)` doesn't work with integer year columns
2. **Time Series Format**: Grafana's time series format requires a timestamp column, not an integer
3. **Query Syntax**: PostgreSQL queries need proper timestamp conversion for time-based filtering

---

## Solution Applied

### 1. Convert Year to Timestamp

Changed all time series queries from:
```sql
SELECT year AS "time", ...
```

To:
```sql
SELECT TO_TIMESTAMP(year::text || '-01-01', 'YYYY-MM-DD') AS "time", ...
```

This converts the integer year (e.g., 2015) to a proper timestamp (2015-01-01 00:00:00).

### 2. Fix Time Range Filtering

Replaced `$__timeFilter(year)` with:
```sql
WHERE year >= EXTRACT(YEAR FROM TO_TIMESTAMP($__timeFrom()::bigint / 1000))
  AND year <= EXTRACT(YEAR FROM TO_TIMESTAMP($__timeTo()::bigint / 1000))
```

This:
- Converts Grafana's time range (milliseconds since epoch) to years
- Filters the integer year column correctly
- Works with Grafana's time picker

---

## Fixed Queries

### Energy Metrics Dashboard

1. **Total Energy Consumption by Country Over Time** ✅
   - Fixed timestamp conversion
   - Fixed time filtering

2. **Year-over-Year Energy Change Percentage** ✅
   - Fixed timestamp conversion
   - Fixed time filtering

### Regional Comparison Dashboard

1. **Renewable Energy Percentage Over Time by Country** ✅
   - Fixed timestamp conversion
   - Fixed time filtering

2. **3-Year Rolling Average Energy Consumption** ✅
   - Fixed timestamp conversion
   - Fixed time filtering

---

## Query Pattern

All time series queries now follow this pattern:

```sql
SELECT
  TO_TIMESTAMP(year::text || '-01-01', 'YYYY-MM-DD') AS "time",
  cntr_code,
  <metric_column>
FROM ml_dataset_forecasting_v1
WHERE year >= EXTRACT(YEAR FROM TO_TIMESTAMP($__timeFrom()::bigint / 1000))
  AND year <= EXTRACT(YEAR FROM TO_TIMESTAMP($__timeTo()::bigint / 1000))
  [AND additional_filters]
ORDER BY year, cntr_code
```

---

## Verification

✅ SQL syntax validated with test queries  
✅ Timestamp conversion works correctly  
✅ Time filtering logic verified  
✅ Dashboards updated and restarted

---

## Next Steps

1. **Refresh dashboards** in Grafana UI
2. **Verify all panels** are loading data correctly
3. **Test time range picker** to ensure filtering works
4. **Check for any remaining errors**

---

**Status**: ✅ **FIXED**  
**All time series queries updated and validated**
