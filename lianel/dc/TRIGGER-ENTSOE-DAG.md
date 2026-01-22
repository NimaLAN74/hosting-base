# Trigger ENTSO-E Ingestion DAG

This guide explains how to trigger the ENTSO-E ingestion DAG to fetch historical electricity data.

## Prerequisites

- Access to the remote host (72.60.80.84)
- Docker and Airflow containers running
- ENTSO-E API token configured in Airflow Variables

## Quick Start

SSH to the remote host and run:

```bash
cd /root/hosting-base && bash lianel/dc/scripts/trigger-entsoe-dag-remote.sh
```

Or manually:

```bash
# 1. Pull latest code
cd /root/hosting-base
git pull origin master

# 2. Trigger the DAG
docker exec dc-airflow-apiserver-1 airflow dags trigger entsoe_ingestion
```

## What This Does

The DAG will:
- Fetch **2 years of historical electricity data** (730 days from yesterday)
- Process **23 countries** in parallel:
  - AT, BE, BG, CZ, DE, DK, EE, ES, FI, FR, HR, HU, IE, IT, LT, LU, LV, NL, PL, PT, RO, SE, SI, SK
- Split data into **7-day chunks** for efficient processing
- Resume from last checkpoint if data already exists

## Expected Duration

- **Several hours** to complete (23 countries × ~104 weeks of data)
- Each country processes independently in parallel
- Progress can be monitored in real-time

## Monitor Progress

### Via Airflow UI
Visit: https://www.lianel.se/airflow/dags/entsoe_ingestion/grid

### Via Command Line
```bash
# Check running DAGs
docker exec dc-airflow-apiserver-1 airflow dags list-runs -d entsoe_ingestion --state running

# Check recent runs
docker exec dc-airflow-apiserver-1 airflow dags list-runs -d entsoe_ingestion --limit 5

# View logs for a specific task
docker exec dc-airflow-apiserver-1 airflow tasks logs entsoe_ingestion <task_id> <run_id>
```

## Verify Data

After completion, check the database:

```bash
docker exec dc-airflow-apiserver-1 psql -h 172.18.0.1 -U postgres -d lianel_energy -c "
SELECT 
    country_code,
    COUNT(*) as total_records,
    MIN(timestamp_utc) as earliest_date,
    MAX(timestamp_utc) as latest_date
FROM fact_electricity_timeseries
GROUP BY country_code
ORDER BY country_code;
"
```

## Troubleshooting

### DAG Not Triggering
- Check Airflow container is running: `docker ps | grep airflow`
- Check Airflow logs: `docker logs dc-airflow-apiserver-1`
- Verify ENTSO-E API token is set: Check Airflow Variables in UI

### No Data Retrieved
- Verify ENTSO-E API token is valid
- Check API rate limits (ENTSO-E has rate limiting)
- Review task logs for specific errors

### Slow Performance
- This is normal - processing 2 years × 23 countries is a large job
- Each country runs in parallel, but each country processes ~104 weeks sequentially
- Consider running during off-peak hours

## Next Steps

After data ingestion completes:
1. Verify data in the Electricity Timeseries page
2. Test date range filters (e.g., "01012024 to 22012026")
3. Check data quality in Grafana dashboards
4. Set up scheduled daily ingestion (already configured at 03:00 UTC)
