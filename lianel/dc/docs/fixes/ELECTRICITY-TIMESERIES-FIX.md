# Electricity Timeseries Fix

## Problem
The `/api/v1/electricity/timeseries` endpoint returns empty data because the `fact_electricity_timeseries` table is empty.

## Root Cause Analysis

### Findings:
1. **DAG Status**: `entsoe_ingestion` DAG exists and is registered
2. **Schedule**: DAG is scheduled to run daily at 03:00 UTC (`schedule='0 3 * * *'`)
3. **API Token**: `ENTSOE_API_TOKEN` Airflow variable does not exist
4. **DAG Behavior**: The DAG handles missing token gracefully (uses `None` if `KeyError`)
5. **Table**: `fact_electricity_timeseries` table exists but is empty

### Possible Reasons:
- DAG hasn't run yet (scheduled for 03:00 UTC daily)
- DAG ran but failed silently
- DAG is paused
- No ENTSO-E API token configured (though DAG handles this)

## Solution

### Option 1: Trigger DAG Manually (Immediate)
```bash
# Trigger the DAG manually
docker exec dc-airflow-apiserver-1 airflow dags trigger entsoe_ingestion

# Or via Airflow UI:
# Go to Airflow UI → DAGs → entsoe_ingestion → Trigger DAG
```

### Option 2: Configure API Token (Recommended)
```bash
# Set ENTSO-E API token (if available)
docker exec dc-airflow-apiserver-1 airflow variables set ENTSOE_API_TOKEN "your-token-here"

# Or via Airflow UI:
# Go to Airflow UI → Admin → Variables → Add new variable
```

### Option 3: Unpause DAG (If Paused)
```bash
# Unpause the DAG
docker exec dc-airflow-apiserver-1 airflow dags unpause entsoe_ingestion
```

## Verification Steps

1. **Check DAG Run Status**:
   ```bash
   docker exec dc-airflow-apiserver-1 airflow dags show entsoe_ingestion
   ```

2. **Check Table Data**:
   ```sql
   SELECT COUNT(*) FROM fact_electricity_timeseries;
   SELECT MIN(timestamp_utc), MAX(timestamp_utc) FROM fact_electricity_timeseries;
   ```

3. **Test API Endpoint**:
   ```bash
   curl -H "Authorization: Bearer <token>" "https://www.lianel.se/api/v1/electricity/timeseries?limit=10"
   ```

## Related Files
- `dags/entsoe_ingestion_dag.py` - DAG definition
- `frontend/src/electricity/ElectricityTimeseries.js` - Frontend component
- `energy-service/src/handlers/entsoe_osm.rs` - API handler
- `energy-service/src/db/queries_entsoe_osm.rs` - Database queries
- `database/migrations/005_add_entsoe_osm_tables.sql` - Table creation

## Notes

- The DAG can run without `ENTSOE_API_TOKEN`, but will only work for public data
- The DAG is scheduled to run daily at 03:00 UTC
- The DAG processes data in batches per country
- Data is stored with conflict resolution (`ON CONFLICT DO NOTHING`)
