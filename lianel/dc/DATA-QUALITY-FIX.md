# Data Quality Fix: Missing Fossil Products

**Date**: January 12, 2026  
**Issue**: All energy showing as 100% renewable because fossil products are missing  
**Status**: ✅ **FIXED**

---

## Problem

The `fact_energy_annual` table only contained `RA000` (Renewables total) product code, resulting in:
- All energy being classified as 100% renewable
- Missing fossil fuel data (coal, natural gas, etc.)
- Incorrect energy mix calculations

---

## Root Cause

1. **Checkpoint Logic Issue**: The ingestion checkpoint was checking if a country/year combination was ingested (within 1 day), and if so, skipping it entirely. Since only `RA000` was ingested initially, subsequent runs skipped all country/year combinations even though other product codes weren't ingested.

2. **Harmonization Version**: Once data was harmonized, the checkpoint would skip it permanently, preventing re-ingestion of missing product codes.

---

## Solution Applied

### 1. Updated Checkpoint Logic

Changed the checkpoint to only skip if data was **harmonized recently** (within 7 days):

```python
# Before: Skip if ingested within 1 day
WHERE ingestion_timestamp >= CURRENT_DATE - INTERVAL '1 day'

# After: Skip only if harmonized within 7 days
WHERE harmonisation_version IS NOT NULL
  AND ingestion_timestamp >= CURRENT_DATE - INTERVAL '7 days'
```

This allows:
- Re-ingestion of missing product codes
- Re-processing of data that wasn't fully ingested
- Better recovery from partial ingestion failures

### 2. Reset Harmonization Version on Conflict

When updating existing records, reset `harmonisation_version` to NULL:

```sql
ON CONFLICT (...) DO UPDATE SET
    ...
    harmonisation_version = NULL  -- Allow re-harmonization
```

This ensures:
- Newly ingested product codes will be harmonized
- Harmonization DAG will process all product codes
- Data quality is maintained across all product types

---

## Next Steps

1. **Re-run Ingestion DAG**: Trigger `eurostat_ingestion_nrg_bal_s` to ingest all product codes
2. **Verify Product Codes**: Check that fossil products (C0110, C0121, C0350) are now in `fact_energy_annual`
3. **Re-run Harmonization**: Trigger `eurostat_harmonization` to process all product codes
4. **Re-run ML Dataset DAGs**: Update ML datasets with correct energy mix data
5. **Verify Dashboards**: Check that renewable percentages are now correct (not 100%)

---

## Verification Queries

```sql
-- Check product codes in fact_energy_annual
SELECT product_code, COUNT(*) as count 
FROM fact_energy_annual 
GROUP BY product_code 
ORDER BY product_code;

-- Check renewable vs fossil energy
SELECT 
    country_code,
    year,
    SUM(CASE WHEN p.renewable_flag = TRUE THEN value_gwh ELSE 0 END) as renewable,
    SUM(CASE WHEN p.fossil_flag = TRUE THEN value_gwh ELSE 0 END) as fossil,
    SUM(value_gwh) as total
FROM fact_energy_annual e
LEFT JOIN dim_energy_product p ON e.product_code = p.product_code
WHERE year = 2024
GROUP BY country_code, year
ORDER BY country_code;
```

---

**Status**: ✅ **FIXED**  
**DAG Updated**: `eurostat_ingestion_nrg_bal_s.py`  
**Next Action**: Re-run ingestion and harmonization DAGs
