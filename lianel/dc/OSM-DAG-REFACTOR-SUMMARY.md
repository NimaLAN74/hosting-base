# OSM DAG Refactor Summary
**Date**: January 15, 2026

---

## Problem

The OSM DAG was experiencing:
- **OOM Kills**: Processes being killed due to memory exhaustion
- **Memory Issues**: Too many regions processing in parallel
- **Rate Limiting**: Overpass API rate limits being hit
- **Long Execution Times**: 6-hour timeout was too long

---

## Solution: Sequential Batch Processing

### New Architecture

**Before**: All 15 regions processed in parallel (max_active_tasks=2)
- Multiple regions extracting features simultaneously
- High memory usage
- Risk of OOM kills

**After**: Regions processed sequentially in batches
- **5 batches** of 3 regions each
- **Batches run sequentially** (one batch after another)
- **Within each batch, regions run sequentially** (one at a time)
- **max_active_tasks=1** (only one region processing at a time)

### Processing Flow

```
Batch 1 (SE11, SE12, SE21)
  ├─ SE11: lookup → extract → store
  ├─ SE12: lookup → extract → store (after SE11 completes)
  └─ SE21: lookup → extract → store (after SE12 completes)

↓ (Batch 1 completes)

Batch 2 (SE22, SE23, DE11)
  ├─ SE22: lookup → extract → store
  ├─ SE23: lookup → extract → store (after SE22 completes)
  └─ DE11: lookup → extract → store (after SE23 completes)

↓ (Batch 2 completes)

... and so on for all 5 batches
```

### Key Changes

1. **Sequential Processing**
   - Only one region processes at a time
   - Prevents memory buildup
   - Reduces OOM risk

2. **Feature Extraction Optimization**
   - Extract features one type at a time (instead of all at once)
   - 1-second delay between feature types
   - Reduces memory usage during extraction

3. **Reduced Timeout**
   - Execution timeout: 6 hours → 2 hours
   - Prevents tasks from running too long

4. **Improved Error Handling**
   - Better XCom retrieval with fallbacks
   - Continue processing other feature types if one fails
   - Better error messages and logging

---

## Benefits

1. **Memory Efficiency**
   - Only one region in memory at a time
   - Features extracted incrementally
   - No memory buildup

2. **Rate Limiting**
   - Sequential processing respects Overpass API limits
   - Built-in delays between requests
   - Less likely to hit rate limits

3. **Reliability**
   - Less likely to experience OOM kills
   - Better error recovery
   - More predictable execution

4. **Monitoring**
   - Easier to track progress (one region at a time)
   - Clear batch boundaries
   - Better visibility into failures

---

## Trade-offs

### Pros
- ✅ Prevents OOM kills
- ✅ Respects API rate limits
- ✅ More reliable execution
- ✅ Better error handling

### Cons
- ⚠️ Longer total execution time (sequential vs parallel)
- ⚠️ Takes longer to process all regions

**Estimated Execution Time**:
- Before: ~1-2 hours (parallel, but with failures)
- After: ~2-4 hours (sequential, but reliable)

---

## Next Steps

1. ✅ DAG refactored and committed
2. ⏳ Deploy to production
3. ⏳ Trigger DAG manually
4. ⏳ Monitor execution
5. ⏳ Verify no OOM kills
6. ⏳ Check data is being stored

---

**Status**: ✅ **REFACTORED AND READY**
