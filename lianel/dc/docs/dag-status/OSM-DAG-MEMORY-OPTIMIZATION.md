# OSM DAG Memory Optimization
**Date**: January 15, 2026

---

## Problem

Even with sequential batch processing, the DAG was still experiencing OOM kills:
- Task killed after ~60-73 seconds (exit_code=-9, SIGKILL)
- Worker memory limit: 1GB
- Issue: Storing all features in memory before processing

---

## Root Cause

The extraction process was:
1. Extract all features for a region (all 8 feature types)
2. Store all features in memory (`all_features` dict)
3. Pass all features via XCom to storage task
4. Storage task processes and stores metrics

**Problem**: Large regions can have thousands of features, consuming hundreds of MB of memory.

---

## Solution: Immediate Processing

**New Approach**:
1. Extract one feature type at a time
2. **Immediately calculate and store metrics** (don't keep features in memory)
3. Only store feature counts in XCom (not full feature data)
4. Explicitly clear memory after each feature type
5. Storage task just verifies data was stored

### Key Changes

1. **Immediate Storage**
   ```python
   # OLD: Store all features, then process later
   all_features[feature_type] = features[feature_type]
   
   # NEW: Process and store immediately
   metrics = osm_client.calculate_feature_metrics(features, area_km2)
   postgres_hook.run(count_sql, ...)  # Store immediately
   all_features[feature_type] = len(features)  # Store count only
   ```

2. **Memory Management**
   ```python
   # Explicitly clear memory
   del features_result
   import gc
   gc.collect()
   ```

3. **XCom Optimization**
   - Before: Stored full feature dictionaries (could be 100s of MB)
   - After: Store only feature counts (few KB)

4. **Storage Task Simplification**
   - Before: Processed all features and stored metrics
   - After: Just verifies metrics were stored

---

## Benefits

1. **Memory Efficiency**
   - Only one feature type in memory at a time
   - Features cleared immediately after processing
   - XCom payload reduced from MB to KB

2. **Faster Processing**
   - No need to wait for all features before storing
   - Parallel processing of extraction and storage

3. **Better Error Recovery**
   - If one feature type fails, others still get stored
   - Partial results are preserved

---

## Expected Results

- ✅ No OOM kills
- ✅ Lower memory usage (~100-200MB peak vs 500-800MB)
- ✅ Faster execution (immediate storage vs batch storage)
- ✅ More reliable (partial results preserved)

---

## Monitoring

Watch for:
- Memory usage in worker container (should stay < 500MB)
- No SIGKILL errors in logs
- Successful completion of extraction tasks
- Data appearing in `fact_geo_region_features` table

---

**Status**: ✅ **OPTIMIZED AND DEPLOYED**
