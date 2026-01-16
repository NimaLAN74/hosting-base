# OSM Feature Extraction DAG - Debug Summary

**Date**: January 14, 2026  
**Issue**: DAG runs successfully but no data is stored in `fact_geo_region_features`

---

## Problem

The OSM feature extraction DAG completes successfully, but:
- `fact_geo_region_features` table is empty (0 records)
- `meta_osm_extraction_log` table is empty (0 records)
- API returns empty results

---

## Root Cause Analysis

### Task Execution Flow

1. **lookup_region** task: ✅ SUCCESS
   - Returns: `{'region_id': 'SE11', 'area_km2': 7072.85, 'status': 'success'}`
   - Geometry exists in database (273KB GeoJSON)

2. **extract_region_features** task: ⚠️ Returns `'status': 'region_not_found'`
   - Log shows: `'status': 'region_not_found'`
   - This means the XCom pull from lookup task failed or returned unexpected format

3. **store_region_metrics** task: ⚠️ Returns `'status': 'no_features'`
   - Log shows: `'status': 'no_features'`
   - This means `all_features` dict is empty

### Code Analysis

**lookup_region function** (lines 85-122):
- Queries database successfully
- Returns dict with `region_id`, `area_km2`, `status`
- Does NOT include `geometry_json` in return value (but it's queried)

**extract_region_features function** (lines 125-182):
- Pulls XCom from lookup task: `ti.xcom_pull(task_ids=[f'region_{region_id.lower()}.lookup_{region_id.lower()}'])`
- Checks: `if not lookup_result or not isinstance(lookup_result, dict)` → returns early if fails
- Re-queries database for geometry (doesn't use XCom geometry)
- Should work, but log shows `'status': 'region_not_found'`

**store_region_metrics function** (lines 185-310):
- Pulls XCom from extract task
- Checks: `if not all_features:` → returns early if empty
- Never reaches database INSERT statements

---

## Likely Issues

1. **XCom retrieval problem**: The `extract_region_features` function may not be correctly pulling the lookup result from XCom
2. **Geometry query issue**: The re-query in `extract_region_features` may be failing silently
3. **Exception handling**: Exceptions may be caught and returning early with `'region_not_found'` status

---

## Next Steps to Fix

1. **Add debug logging** to see what XCom actually contains
2. **Check if geometry_json is too large** for XCom (273KB may be too large)
3. **Verify task_id format** matches between XCom push and pull
4. **Add exception logging** to see if errors are being swallowed
5. **Test with a smaller region** to see if it's a size issue

---

## Immediate Fix

The `extract_region_features` function should handle the case where geometry_json might be None or empty, and should log more details about what's happening.

**Recommended fix**: Add better error handling and logging to identify the exact failure point.
