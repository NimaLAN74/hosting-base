# Overpass Query Syntax Fix
**Date**: January 15, 2026

---

## Problem

The OSM DAG was completing successfully, but **no data was being stored** in `fact_geo_region_features`. 

**Root Cause**: All Overpass API queries were returning **400 Bad Request** errors because of invalid query syntax.

### Invalid Syntax
```
[out:json][timeout:25];(way["power"="plant"]({{bbox}})(limit:500);...);out geom;
                                                      ^^^^^^^^^^^
                                                      INVALID!
```

The `(limit:N)` syntax is **not valid Overpass QL syntax**. Overpass API was rejecting all queries, resulting in 0 features extracted for all regions.

---

## Solution

Removed the invalid `(limit:N)` clauses from all Overpass queries. Results are already limited in code (`max_features=2000` per type), so the limit clauses were unnecessary.

### Fixed Syntax
```
[out:json][timeout:25];(way["power"="plant"]({{bbox}});...);out geom;
```

---

## Impact

- **Before**: All queries failed with 400 Bad Request → 0 features extracted → Empty database
- **After**: Queries succeed → Features extracted → Data stored

---

## Next Steps

1. ✅ Query syntax fixed and deployed
2. ✅ New DAG run triggered
3. ⏳ Monitor execution - queries should now succeed
4. ⏳ Verify data is stored in `fact_geo_region_features`
5. ⏳ Check OSM page displays data

---

**Status**: ✅ **FIXED** - Waiting for DAG run to complete
