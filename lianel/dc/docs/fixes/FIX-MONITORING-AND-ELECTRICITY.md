# Fix for /monitoring Redirect and Electricity Timeseries Issues

## Issue 1: /monitoring Redirects to /login

### Root Cause
Nginx was redirecting `/monitoring` to `/monitoring/` (trailing slash), which matched the oauth2-proxy location block instead of the React app block.

### Fix Applied
✅ Added `proxy_redirect off;` to the `/monitoring` location block in `nginx/config/nginx.conf` (line 131)

### If Still Redirecting

1. **Verify nginx config is updated on remote host:**
   ```bash
   cd /root/hosting-base/lianel/dc
   grep -A 15 "location = /monitoring" nginx/config/nginx.conf | grep "proxy_redirect"
   ```
   Should show: `proxy_redirect off;`

2. **If missing, add it manually:**
   ```bash
   sed -i '/location = \/monitoring {/,/^[[:space:]]*}/ { 
       /proxy_cache_bypass/a\
           proxy_redirect off;
   }' nginx/config/nginx.conf
   ```

3. **Reload nginx:**
   ```bash
   docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload
   ```

4. **Check if frontend needs rebuild:**
   The frontend needs the `keycloakReady` check fix. Check if it's deployed:
   ```bash
   docker exec lianel-frontend grep -r 'keycloakReady' /app/build/static/js/*.js 2>/dev/null | head -1
   ```
   If not found, the frontend pipeline needs to run to deploy the latest code.

5. **Test the redirect:**
   ```bash
   curl -I 'https://www.lianel.se/monitoring' | grep -E '< HTTP|< Location'
   ```
   Should NOT show a 301 redirect to `/monitoring/`

## Issue 2: Electricity Timeseries Shows "No Data"

### Possible Causes
1. `ENTSOE_API_TOKEN` Airflow Variable is not set
2. DAG ran but failed to insert data (check task logs)
3. Table is empty (DAG hasn't run or failed silently)

### Diagnostic Steps

Run the diagnostic script:
```bash
cd /root/hosting-base/lianel/dc
bash scripts/fix-monitoring-and-electricity.sh
```

### Manual Checks

1. **Check if table has data:**
   ```bash
   docker exec dc-airflow-apiserver-1 psql -h 172.18.0.1 -U airflow -d lianel_energy -c "SELECT COUNT(*) FROM fact_electricity_timeseries;"
   ```

2. **Check if ENTSOE_API_TOKEN is set:**
   ```bash
   docker exec dc-airflow-apiserver-1 airflow variables get ENTSOE_API_TOKEN
   ```

3. **If token is missing, set it:**
   ```bash
   # Get token from: https://transparency.entsoe.eu/
   docker exec dc-airflow-apiserver-1 airflow variables set ENTSOE_API_TOKEN 'your_token_here'
   ```

4. **Check DAG status:**
   ```bash
   docker exec dc-airflow-apiserver-1 airflow dags state entsoe_ingestion
   ```

5. **Check latest DAG run:**
   ```bash
   docker exec dc-airflow-apiserver-1 airflow dags list-runs -d entsoe_ingestion --state success --no-backfill | head -5
   ```

6. **Check task logs for errors:**
   ```bash
   docker exec dc-airflow-apiserver-1 airflow tasks list entsoe_ingestion
   # Then check logs for specific task
   docker exec dc-airflow-apiserver-1 airflow tasks log entsoe_ingestion ingest_entsoe_data $(date +%Y-%m-%d) 1
   ```

7. **Trigger DAG manually:**
   ```bash
   docker exec dc-airflow-apiserver-1 airflow dags trigger entsoe_ingestion
   ```

8. **Test API endpoint:**
   ```bash
   curl 'https://www.lianel.se/api/v1/electricity/timeseries?country_code=SE&start_date=2024-01-01&end_date=2024-12-31'
   ```

## Quick Fix Commands

Run these on the remote host:

```bash
cd /root/hosting-base/lianel/dc

# 1. Fix nginx config (if not already fixed)
sed -i '/location = \/monitoring {/,/^[[:space:]]*}/ { 
    /proxy_cache_bypass/a\
        proxy_redirect off;
}' nginx/config/nginx.conf

# 2. Reload nginx
docker exec nginx-proxy nginx -t && docker exec nginx-proxy nginx -s reload

# 3. Run diagnostic script
bash scripts/fix-monitoring-and-electricity.sh

# 4. If ENTSOE_API_TOKEN is missing, set it (replace with your actual token)
# docker exec dc-airflow-apiserver-1 airflow variables set ENTSOE_API_TOKEN 'your_token_here'

# 5. Trigger DAG
# docker exec dc-airflow-apiserver-1 airflow dags trigger entsoe_ingestion
```

## Expected Results

After fixes:
- ✅ `/monitoring` should go directly to React app (no redirect to `/login`)
- ✅ Electricity timeseries should show data if:
  - `ENTSOE_API_TOKEN` is set
  - DAG has run successfully
  - Data was inserted into `fact_electricity_timeseries` table
