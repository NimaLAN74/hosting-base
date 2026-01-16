# OSM Feature Extraction - Rate Limiting Fix

## Problem
The Overpass API public endpoint (`https://overpass-api.de/api/interpreter`) was returning:
- **429 Too Many Requests**: Rate limiting due to too many concurrent requests
- **504 Gateway Timeout**: Queries taking too long or server overloaded

This caused the OSM feature extraction DAG to fail, leaving `fact_geo_region_features` empty.

## Root Causes
1. **High request frequency**: Processing multiple regions in parallel, each making 8 feature type requests
2. **Large bounding boxes**: Some regions (e.g., SE23 with 34,572 km²) generate very large queries
3. **No rate limit handling**: The client didn't properly handle 429/504 errors
4. **Insufficient delays**: Only 1 second delay between requests

## Fixes Applied

### 1. Enhanced Error Handling (`osm_client.py`)
- **429 (Rate Limited)**: Wait up to 60 seconds with exponential backoff before retry
- **504 (Gateway Timeout)**: Wait up to 30 seconds with exponential backoff
- **Increased retries**: From 3 to 5 attempts
- **Better error messages**: Log response body for debugging

### 2. Rate Limiting Delays
- **After successful request**: 3 seconds delay between feature types
- **After failed request**: 10 seconds delay (indicates rate limiting)

### 3. Reduced Parallelism (`osm_feature_extraction_dag.py`)
- **max_active_tasks**: Limited to 3 concurrent regions (down from unlimited)
- This reduces simultaneous Overpass API requests

## Current Status
The DAG has been updated and re-triggered. With the new rate limiting handling:
- Failed requests will wait longer before retrying
- Fewer regions processed simultaneously
- Better chance of success even with rate limits

## Monitoring
Check DAG status:
```bash
# Check latest run
docker exec dc-airflow-scheduler-1 airflow dags list-runs -d osm_feature_extraction --state running

# Check logs for rate limiting
docker exec dc-airflow-worker-1 find /opt/airflow/logs -path '*osm_feature_extraction*' -name '*.log' -exec grep -l "429\|504\|Rate limited" {} \;
```

## Alternative Solutions (if issues persist)

### Option 1: Use Different Overpass Endpoint
The public Overpass API has multiple instances. You can switch to a less busy one:

```python
# In osm_client.py, change:
OVERPASS_API_URL = "https://overpass.kumi.systems/api/interpreter"  # Alternative endpoint
# or
OVERPASS_API_URL = "https://overpass.openstreetmap.fr/api/interpreter"  # French instance
```

### Option 2: Reduce Feature Types
Process fewer feature types per run, or split into separate DAGs:
- Energy infrastructure (power_plant, power_generator, power_substation)
- Buildings (residential_building, commercial_building)
- Infrastructure (railway_station, airport, industrial_area)

### Option 3: Process Regions Sequentially
Set `max_active_tasks=1` to process one region at a time (slower but avoids rate limits)

### Option 4: Self-Hosted Overpass Instance
For production use with high volume, consider hosting your own Overpass API instance.

## Next Steps
1. Monitor the current DAG run (may take 30-60 minutes with rate limiting delays)
2. If still failing, try Option 1 (different endpoint)
3. If successful, data should appear in `fact_geo_region_features` table

## Verification
Once the DAG completes successfully:
```sql
SELECT COUNT(*) FROM fact_geo_region_features;
SELECT region_id, feature_name, feature_value 
FROM fact_geo_region_features 
ORDER BY region_id, feature_name 
LIMIT 20;
```
