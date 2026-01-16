# Electricity Timeseries Empty Data Issue

## Problem
The `/api/v1/electricity/timeseries` endpoint returns empty data: `{"data":[],"total":0}`

## Root Cause
The `fact_electricity_timeseries` table is empty because:
- The `entsoe_ingestion` DAG exists and is registered
- The DAG inserts into `fact_electricity_timeseries` table
- The DAG likely hasn't run successfully or hasn't run at all

## Investigation

### API Endpoint
- **Endpoint**: `/api/v1/electricity/timeseries`
- **Response**: `{"data":[],"total":0,"limit":1,"offset":0}`
- **Status**: Working correctly, but table is empty

### DAG Status
- **DAG ID**: `entsoe_ingestion`
- **Status**: Registered in Airflow
- **Table**: `fact_electricity_timeseries` (created in migration 005)
- **Insert Logic**: DAG inserts records in batches of 100

### Frontend
- **Component**: `ElectricityTimeseries.js`
- **Authentication**: Uses `authenticatedFetch` with Bearer token
- **API Call**: `/api/v1/electricity/timeseries?limit=1000`

## Solution

### Immediate Action
1. **Check DAG Run Status**:
   ```bash
   docker exec dc-airflow-apiserver-1 airflow dags list-runs -d entsoe_ingestion
   ```

2. **Trigger DAG Manually** (if not running):
   ```bash
   docker exec dc-airflow-apiserver-1 airflow dags trigger entsoe_ingestion
   ```

3. **Verify Table Has Data**:
   ```sql
   SELECT COUNT(*) FROM fact_electricity_timeseries;
   SELECT MIN(timestamp_utc), MAX(timestamp_utc) FROM fact_electricity_timeseries;
   ```

### Verification
- Check DAG run history for failures
- Verify ENTSO-E API token is configured (`ENTSOE_API_TOKEN` Airflow variable)
- Check database connection is working
- Verify table exists and schema is correct

## Related Files
- `dags/entsoe_ingestion_dag.py` - DAG definition
- `frontend/src/electricity/ElectricityTimeseries.js` - Frontend component
- `energy-service/src/handlers/entsoe_osm.rs` - API handler
- `energy-service/src/db/queries_entsoe_osm.rs` - Database queries
- `database/migrations/005_add_entsoe_osm_tables.sql` - Table creation
