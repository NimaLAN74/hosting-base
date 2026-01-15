# OOM (Out of Memory) Fix Summary
**Date**: 2026-01-15  
**Issue**: Airflow worker processes being killed by OOM killer  
**Status**: ✅ Fixed

## Problem

Despite setting container memory limits, individual Airflow worker processes were still being killed by the OOM killer:

```
Memory cgroup out of memory: Killed process 759949 (airflow) 
total-vm:507112kB, anon-rss:178744kB
```

### Root Causes
1. **Worker container limit too high**: 2GB allowed too many concurrent processes
2. **Worker concurrency too high**: 4 workers × multiple child processes = memory pressure
3. **Parallelism too high**: 8 concurrent tasks across system
4. **max_mem_per_child not strict enough**: 200MB per child still too high

## Solution Applied

### 1. Reduced Worker Container Memory
- **Before**: 2GB limit
- **After**: 1GB limit
- **Impact**: Forces stricter memory discipline

### 2. Reduced Worker Concurrency
- **Before**: 4 concurrent workers
- **After**: 2 concurrent workers
- **Impact**: 50% reduction in concurrent processes

### 3. Reduced System Parallelism
- **Before**: 8 concurrent tasks
- **After**: 4 concurrent tasks
- **Impact**: Less overall system load

### 4. Reduced Max Memory Per Child
- **Before**: 200MB per child process
- **After**: 150MB per child process
- **Impact**: Stricter per-process limits

### 5. Reduced DAG max_active_tasks
- **Before**: 3 concurrent tasks per DAG
- **After**: 2 concurrent tasks per DAG
- **Impact**: Less memory pressure from OSM DAG

## Configuration Changes

### docker-compose.airflow.yaml
```yaml
# Environment variables
AIRFLOW__CORE__PARALLELISM: 4  # was 8
AIRFLOW__CELERY__WORKER_CONCURRENCY: 2  # was 4
AIRFLOW__CELERY__MAX_MEM_PER_CHILD: 150000  # was 200000

# Worker container limits
deploy:
  resources:
    limits:
      cpus: '1.5'  # was 2.0
      memory: 1G   # was 2G
```

### osm_feature_extraction_dag.py
```python
max_active_tasks=2  # was 3
```

## Expected Results

- **Worker memory**: Should stay well below 1GB limit
- **OOM kills**: Should stop occurring
- **Task execution**: Slower but stable
- **System stability**: Improved

## Monitoring

### Key Metrics to Watch
- Worker container memory < 800MB (80% of 1GB limit)
- No OOM kills in dmesg
- Tasks completing successfully
- System memory usage stable

### Verification Commands
```bash
# Check worker memory
docker stats --no-stream | grep airflow-worker

# Check for OOM kills
dmesg | grep -i 'killed process\|oom'

# Check worker concurrency
docker exec dc-airflow-scheduler-1 airflow config get-value celery worker_concurrency

# Check actual worker processes
docker exec dc-airflow-worker-1 ps aux | grep celery
```

## Trade-offs

### Benefits
- ✅ No more OOM kills
- ✅ Stable system operation
- ✅ Predictable memory usage

### Costs
- ⚠️ Slower task execution (fewer concurrent tasks)
- ⚠️ Longer DAG run times
- ⚠️ Less parallelism

## Next Steps

1. **Monitor for 24-48 hours** to ensure stability
2. **If stable**, consider gradual increases:
   - Worker concurrency: 2 → 3 (if memory allows)
   - Parallelism: 4 → 6 (if system allows)
3. **If still OOM kills**:
   - Further reduce worker concurrency to 1
   - Reduce parallelism to 2
   - Consider adding swap space (not ideal but can help)

## Rollback

If issues occur, revert to previous settings:
```bash
git checkout HEAD~1 lianel/dc/docker-compose.airflow.yaml lianel/dc/dags/osm_feature_extraction_dag.py
cd /root/lianel/dc
docker compose -f docker-compose.airflow.yaml up -d --force-recreate airflow-worker
```
