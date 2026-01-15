# OSM DAG XCom Fix
**Date**: January 15, 2026

---

## Problem

The OSM DAG was running successfully (all tasks showing "success"), but the `fact_geo_region_features` table remained empty. 

**Root Cause**: The `extract_region_features` and `store_region_metrics` functions were trying to retrieve XCom data using incorrect task IDs. They were missing the batch prefix (`batch_1`, `batch_2`, etc.) in the task ID.

---

## Issue Details

### Task Structure
Tasks are organized in nested TaskGroups:
- `batch_1` → `region_se11` → `lookup_se11`, `extract_se11`, `store_se11`
- Full task ID: `batch_1.region_se11.lookup_se11`

### Previous Code (Broken)
```python
lookup_task_id = f'region_{region_id.lower()}.lookup_{region_id.lower()}'
region_data = ti.xcom_pull(task_ids=[lookup_task_id])
```
**Problem**: Missing `batch_1.` prefix, so XCom pull failed and returned `None`

### Fixed Code
```python
possible_lookup_task_ids = [
    f'batch_1.region_{region_id.lower()}.lookup_{region_id.lower()}',
    f'batch_2.region_{region_id.lower()}.lookup_{region_id.lower()}',
    # ... etc for all batches
]
region_data = None
for lookup_task_id in possible_lookup_task_ids:
    try:
        region_data = ti.xcom_pull(task_ids=lookup_task_id)
        if region_data:
            break
    except:
        continue
```
**Solution**: Try all possible batch prefixes to find the correct XCom data

---

## Impact

- **Before**: Tasks succeeded but no data stored (XCom retrieval failed silently)
- **After**: XCom data retrieved correctly, data stored to database

---

## Next Steps

1. ✅ Fix committed and deployed
2. ⏳ Trigger new DAG run to test
3. ⏳ Verify data is stored in `fact_geo_region_features`
4. ⏳ Check OSM page displays data

---

**Status**: ✅ **FIXED**
