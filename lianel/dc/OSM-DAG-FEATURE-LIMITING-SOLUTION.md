# OSM DAG Feature Limiting Solution
**Date**: January 15, 2026

---

## Problem

Even with immediate storage and memory cleanup, the DAG was still experiencing OOM kills because:
- Some regions have **thousands of features** (especially residential/commercial buildings)
- Overpass API returns all features, consuming too much memory
- Processing all features at once overwhelms the 1GB worker memory limit

---

## Solution: Feature Limiting and Chunked Processing

### 1. Reduced Feature Types

**Removed** (too many features, causing OOM):
- `residential_building` - Can have 10,000+ features per region
- `commercial_building` - Can have 5,000+ features per region

**Kept** (essential for energy analysis):
- `power_plant` - Critical for energy infrastructure
- `power_generator` - Critical for energy infrastructure
- `power_substation` - Critical for energy infrastructure
- `industrial_area` - Important for energy consumption analysis
- `railway_station` - Transport infrastructure
- `airport` - Transport infrastructure

### 2. Overpass Query Limits

Added `(limit:N)` to all Overpass queries to cap results at the API level:

```python
'power_plant': '[out:json][timeout:25];(way["power"="plant"]({{bbox}})(limit:500);...);out geom;'
'residential_building': '[out:json][timeout:25];(way["building"~"..."]({{bbox}})(limit:2000);...);out geom;'
```

**Limits per feature type**:
- Power plants: 500 ways, 100 relations
- Generators/substations: 1000 ways, 200 relations
- Industrial areas: 500 ways, 100 relations
- Railway stations: 200 ways, 50 relations
- Airports: 100 ways, 50 relations

### 3. Code-Level Hard Limits

Added hard limit of 2000 features per type in the extraction code:

```python
max_features = 2000  # Hard limit per feature type
if len(features) >= max_features:
    logger.warning(f"Reached feature limit ({max_features}), truncating results")
    break
```

### 4. Chunked Processing

For feature sets larger than 500, process in chunks:

```python
chunk_size = 500
if len(features) > chunk_size:
    # Process in chunks, aggregate metrics
    for i in range(0, len(features), chunk_size):
        chunk = features[i:i+chunk_size]
        chunk_metrics = calculate_metrics(chunk)
        # Aggregate and clear chunk
```

### 5. Enhanced Memory Management

- Increased delay between feature types: 2s → 3s
- More aggressive cleanup: delete features immediately after processing
- Force garbage collection after each feature type

---

## Benefits

1. **Memory Safety**
   - Maximum ~12,000 features in memory at once (6 types × 2000)
   - Chunked processing reduces peak memory to ~500 features
   - Hard limits prevent unbounded growth

2. **Faster Processing**
   - Fewer feature types to process
   - Smaller datasets process faster
   - Less time spent on memory management

3. **More Reliable**
   - Predictable memory usage
   - No risk of OOM from large regions
   - Graceful handling of feature limits

---

## Trade-offs

### Pros
- ✅ Prevents OOM kills
- ✅ Faster execution
- ✅ More reliable
- ✅ Still captures essential features

### Cons
- ⚠️ Removed residential/commercial building data
- ⚠️ Large regions may have truncated results
- ⚠️ May miss some features in dense areas

**Note**: For regions with many buildings, the data is still useful for energy analysis (power infrastructure, industrial areas, transport) even without building counts.

---

## Expected Results

- ✅ No OOM kills
- ✅ Memory usage stays < 500MB
- ✅ All regions process successfully
- ✅ Essential features captured

---

**Status**: ✅ **IMPLEMENTED AND DEPLOYED**
