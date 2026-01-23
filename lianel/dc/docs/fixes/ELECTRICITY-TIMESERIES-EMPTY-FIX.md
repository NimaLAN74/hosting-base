# Electricity Timeseries Data Empty - Investigation & Fix

## Issue
The Electricity Timeseries page shows no data even though the API endpoint exists.

## Root Cause
The `fact_electricity_timeseries` table is likely empty because:
1. The ENTSO-E ingestion DAG (`entsoe_ingestion`) may not have run successfully
2. The DAG may not have been triggered
3. There may be data ingestion errors

## Investigation Steps

### 1. Check if table exists and has data
```bash
# On remote host
docker exec -i <postgres-container> psql -U airflow -d lianel_energy -c "SELECT COUNT(*) FROM fact_electricity_timeseries;"
```

### 2. Check ENTSO-E DAG status in Airflow
- Go to https://airflow.lianel.se
- Check the `entsoe_ingestion` DAG
- Verify it has run successfully
- Check for any errors in the task logs

### 3. Manually trigger the DAG
- In Airflow UI, click on `entsoe_ingestion` DAG
- Click "Trigger DAG" button
- Monitor the execution

### 4. Check DAG configuration
- Verify ENTSO-E API credentials are set in Airflow variables/secrets
- Check if the DAG schedule is correct (currently: daily at 03:00 UTC)
- Verify the DAG is enabled

## Fix Options

### Option 1: Trigger DAG Manually
1. Log into Airflow UI
2. Find `entsoe_ingestion` DAG
3. Click "Trigger DAG"
4. Wait for completion
5. Refresh Electricity Timeseries page

### Option 2: Check DAG for Errors
1. Review DAG logs in Airflow
2. Fix any configuration issues (API keys, database connections)
3. Re-run the DAG

### Option 3: Verify Data Ingestion
1. Check if ENTSO-E API is accessible
2. Verify database connection from Airflow
3. Check if table schema matches DAG expectations

## Expected Data
Once the DAG runs successfully, the `fact_electricity_timeseries` table should contain:
- `timestamp_utc`: UTC timestamp
- `country_code`: ISO country code (e.g., 'SE', 'DE')
- `production_type`: Production type code (e.g., 'B16' for Solar)
- `load_mw`: Load in megawatts
- `generation_mw`: Generation in megawatts
- `resolution`: Time resolution (e.g., 'PT60M' for hourly)

## Next Steps
1. Verify DAG has run and data exists
2. If data exists but page still shows empty, check API endpoint
3. If API endpoint works but frontend shows empty, check frontend code
