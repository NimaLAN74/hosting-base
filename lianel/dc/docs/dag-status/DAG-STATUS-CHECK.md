# DAG Status Check
**Date**: January 15, 2026 - 17:25 UTC

---

## ✅ OSM DAG is Running!

**DAG Run ID**: `manual__2026-01-15T17:25:17.946700+00:00`  
**Status**: Running  
**Current State**: Tasks queued and executing

### Task Progress

1. ✅ **batch_1.region_se11.lookup_se11** - **SUCCESS** (completed in 7.1 seconds)
2. ⏳ **batch_1.region_se11.extract_se11** - **QUEUED/RUNNING**
3. ⏳ **batch_1.region_se11.store_se11** - **SCHEDULED** (waiting for extract)

---

## How to View DAGs in Airflow UI

1. **Access Airflow UI**: https://airflow.lianel.se
2. **Login** with your credentials
3. **View DAGs**:
   - You'll see all DAGs listed
   - Look for `osm_feature_extraction` 
   - The circle should be GREEN/YELLOW if running
   - Click on the DAG name to see details

4. **View DAG Run**:
   - Click on `osm_feature_extraction` DAG
   - Click on the latest run (should show "Running")
   - View task status (green = success, yellow = running, red = failed)

---

## Current Execution Status

The OSM DAG is processing regions sequentially in batches:
- **Batch 1**: SE11, SE12, SE21 (one at a time)
- **Batch 2**: SE22, SE23, DE11 (after Batch 1 completes)
- **Batch 3**: DE12, DE21, DE22
- **Batch 4**: DE30, FR10, FR21
- **Batch 5**: FR22, FR30, FR41

**Current Task**: Processing SE11 region

---

## Expected Timeline

- **Per Region**: ~2-5 minutes (depending on OSM data size)
- **Per Batch** (3 regions): ~6-15 minutes
- **Total** (5 batches, 15 regions): ~30-75 minutes

---

## Monitoring

You can monitor progress:
- **Airflow UI**: https://airflow.lianel.se
- **Logs**: Check task logs for each region
- **Database**: Check `fact_geo_region_features` table for new data

---

**Status**: ✅ **DAG IS RUNNING** - Processing Batch 1, Region SE11
