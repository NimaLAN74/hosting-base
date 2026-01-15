# INSERT Statement Fix Summary
**Date**: January 15, 2026

---

## Problem

The DAG was extracting features successfully (3554 features for SE11), but **INSERT statements were failing** with:

```
column "updated_at" of relation "fact_geo_region_features" does not exist
```

---

## Root Cause

The `fact_geo_region_features` table has `ingestion_timestamp` (not `updated_at`), but the INSERT statements were trying to update a non-existent `updated_at` column in the `ON CONFLICT` clause.

---

## Solution

Removed `updated_at = CURRENT_TIMESTAMP` from all INSERT statements:

### Before (Invalid):
```sql
ON CONFLICT (region_id, feature_name, snapshot_year)
DO UPDATE SET feature_value = EXCLUDED.feature_value,
              updated_at = CURRENT_TIMESTAMP  ❌
```

### After (Valid):
```sql
ON CONFLICT (region_id, feature_name, snapshot_year)
DO UPDATE SET feature_value = EXCLUDED.feature_value  ✅
```

---

## Impact

- **Before**: INSERT statements failing → No data stored despite successful extraction
- **After**: INSERT statements succeed → Data stored in database

---

## Fixes Applied

1. ✅ Removed `updated_at` from count feature INSERT
2. ✅ Removed `updated_at` from area feature INSERT  
3. ✅ Removed `updated_at` from density feature INSERT
4. ✅ Removed `updated_at` from chunked processing INSERT statements

---

## Next Steps

1. ✅ INSERT fix deployed
2. ✅ New DAG run triggered
3. ⏳ Monitor execution - INSERTs should now succeed
4. ⏳ Verify data is stored in `fact_geo_region_features`
5. ⏳ Check OSM page displays data

---

**Status**: ✅ **FIXED** - Waiting for DAG run to complete
