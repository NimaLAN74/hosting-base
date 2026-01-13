# Airflow Internal Error Fix

**Date**: January 12, 2026  
**Issue**: Airflow apiserver returning internal error (PendingRollbackError)  
**Status**: ✅ **FIXED**

---

## Problem

Airflow apiserver was returning internal errors:
```
sqlalchemy.exc.PendingRollbackError: Can't reconnect until invalid transaction is rolled back.
```

**Root Cause**: Database connection pool issues - stale connections with uncommitted transactions.

---

## Solution Applied

### 1. Added Connection Pool Configuration

Added SQLAlchemy connection pool settings to `docker-compose.airflow.yaml`:

```yaml
AIRFLOW__DATABASE__SQL_ALCHEMY_POOL_SIZE: 5
AIRFLOW__DATABASE__SQL_ALCHEMY_MAX_OVERFLOW: 10
AIRFLOW__DATABASE__SQL_ALCHEMY_POOL_PRE_PING: 'true'
AIRFLOW__DATABASE__SQL_ALCHEMY_POOL_RECYCLE: 3600
```

**Settings Explained**:
- `POOL_SIZE: 5` - Base number of connections in pool
- `MAX_OVERFLOW: 10` - Additional connections allowed beyond pool_size
- `POOL_PRE_PING: true` - Test connections before using (prevents stale connections)
- `POOL_RECYCLE: 3600` - Recycle connections after 1 hour (prevents timeout issues)

### 2. Restarted All Airflow Services

Restarted all Airflow containers to apply new configuration:
- airflow-apiserver
- airflow-scheduler
- airflow-dag-processor
- airflow-triggerer
- airflow-worker

---

## Verification

✅ **All services healthy**:
- dc-airflow-apiserver-1: healthy
- dc-airflow-scheduler-1: healthy
- dc-airflow-dag-processor-1: healthy
- dc-airflow-triggerer-1: healthy
- dc-airflow-worker-1: healthy

✅ **No errors in logs**: PendingRollbackError resolved

✅ **API responding**: `/api/v2/version` endpoint working

---

## Files Modified

- `lianel/dc/docker-compose.airflow.yaml` - Added connection pool environment variables

---

## Prevention

The connection pool settings will:
- Prevent stale connections by testing before use (`pool_pre_ping`)
- Recycle connections regularly to avoid timeout issues (`pool_recycle`)
- Limit connection pool size to prevent database overload
- Allow overflow for peak usage

---

**Status**: ✅ **FIXED**  
**All Airflow services operational**
