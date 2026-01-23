# ENTSO-E Ingestion DAG Triggered

**Date**: 2026-01-19  
**Action**: Manually triggered `entsoe_ingestion` DAG

## DAG Information

- **DAG ID**: `entsoe_ingestion`
- **Schedule**: Daily at 03:00 UTC (automatic)
- **Status**: ✅ Triggered and queued
- **Run ID**: `manual__2026-01-19T12:51:32.334911+00:00`

## DAG Configuration

### Countries Processed
The DAG processes data for 24 EU countries:
- AT, BE, BG, CZ, DE, DK, EE, ES, FI, FR
- HR, HU, IE, IT, LT, LU, LV, NL, PL, PT, RO
- SE, SI, SK

### API Configuration
- **API Token**: Optional (from Airflow Variable `ENTSOE_API_TOKEN`)
- **Base URL**: https://web-api.tp.entsoe.eu/api
- **Note**: DAG can run without API token (some endpoints are public)

### Data Ingested
- **Load Data**: Electricity demand per country/bidding zone
- **Generation Data**: Electricity generation by production type
- **Frequency**: Hourly or 15-minute intervals
- **Storage**: `fact_electricity_timeseries` table

## Monitoring

### Check DAG Status
```bash
docker exec dc-airflow-apiserver-1 airflow dags list-runs --dag-id entsoe_ingestion --limit 5
```

### Check DAG Logs
```bash
# Worker logs
docker logs dc-airflow-worker-1 --tail 100 | grep entsoe

# Scheduler logs
docker logs dc-airflow-scheduler-1 --tail 100 | grep entsoe
```

### Check Data in Database
```bash
# Via energy service
docker exec lianel-energy-service sh -c 'PGPASSWORD=$POSTGRES_PASSWORD psql -h 172.18.0.1 -p 5432 -U airflow -d lianel_energy -c "SELECT COUNT(*) FROM fact_electricity_timeseries;"'
```

### Via API
```bash
curl https://www.lianel.se/api/v1/electricity/timeseries?limit=10
```

## Expected Behavior

1. **DAG Execution**:
   - DAG will process each country sequentially
   - For each country, it will:
     - Check last ingestion date (checkpoint)
     - Plan date ranges to ingest
     - Ingest data in 7-day chunks
     - Log ingestion results

2. **Data Population**:
   - Data will be inserted into `fact_electricity_timeseries`
   - Records include: timestamp, country, bidding zone, production type, load, generation
   - Duplicate records are skipped (ON CONFLICT DO NOTHING)

3. **Time to Complete**:
   - Initial run: May take 1-4 hours (30 days of data for 24 countries)
   - Subsequent runs: Faster (only new data)
   - Depends on API rate limits and data availability

## Troubleshooting

### If DAG Fails

1. **Check Logs**:
   ```bash
   docker logs dc-airflow-worker-1 --tail 200 | grep -A 10 entsoe
   ```

2. **Common Issues**:
   - **API Rate Limiting**: ENTSO-E may rate limit requests
   - **Missing API Token**: Some endpoints require authentication
   - **Network Issues**: Connection to ENTSO-E API
   - **Database Connection**: Verify PostgresHook connection

3. **Retry**:
   - DAG can be manually triggered again
   - Failed tasks can be retried individually in Airflow UI

### If No Data Appears

1. **Check DAG Status**: Ensure DAG completed successfully
2. **Check Database**: Verify records were inserted
3. **Check Date Range**: DAG starts from 30 days ago (initial load)
4. **Check API Response**: ENTSO-E may not have data for all dates

## Next Steps

1. **Monitor DAG Execution**:
   - Check Airflow UI: https://airflow.lianel.se
   - Navigate to `entsoe_ingestion` DAG
   - Monitor task execution

2. **Wait for Completion**:
   - DAG may take 1-4 hours for initial run
   - Check progress in Airflow UI

3. **Verify Data**:
   - Once DAG completes, check API endpoint
   - Data should appear in Electricity Timeseries page

4. **Schedule**:
   - DAG will run automatically daily at 03:00 UTC
   - No manual intervention needed after initial run

## Notes

- **API Token**: Optional but recommended for better rate limits
- **Data Availability**: ENTSO-E data is typically available with 1-day delay
- **Initial Load**: First run processes last 30 days of data
- **Incremental**: Subsequent runs only process new data since last run
