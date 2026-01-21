# Electricity Timeseries "No Data Available" - Root Cause & Fix

## Problem
The Electricity Timeseries page shows "No data available" even though:
- ✅ The API endpoint is working correctly
- ✅ The database table exists with correct schema
- ✅ The DAG runs show "success" status

## Root Cause

**The `ENTSOE_API_TOKEN` Airflow Variable is NOT set.**

### Evidence:
1. **Table is empty**: `SELECT COUNT(*) FROM fact_electricity_timeseries;` → `0 records`
2. **Token missing**: `airflow variables get ENTSOE_API_TOKEN` → `Variable ENTSOE_API_TOKEN does not exist`
3. **DAG runs but no data**: All tasks show "success" but `meta_entsoe_ingestion_log` is empty
4. **API returns empty**: `/api/v1/electricity/timeseries` → `{"data": [], "total": 0}`

### Why DAG Shows "Success" But No Data:
- The DAG code handles missing token gracefully (sets `api_token = None`)
- ENTSO-E API calls without token return empty data or fail silently
- Tasks complete without errors but insert 0 records
- The DAG marks tasks as "success" even when no data is retrieved

## Solution

### Step 1: Get ENTSO-E API Token

1. **Register on ENTSO-E Transparency Platform**:
   - Visit: https://transparency.entsoe.eu/
   - Click "Registration" or "Sign Up"
   - Create an account with your email
   - Verify your email address

2. **Request API Access**:
   - Log in to your account
   - Navigate to "Web Services" or "API Access" section
   - Request API access (may require approval)
   - Once approved, you'll receive an API token

### Step 2: Set Token in Airflow

**Option A: Using Script (Recommended)**
```bash
# On remote host
cd /root/lianel/dc
./scripts/set-entsoe-token.sh "your-token-here"
```

**Option B: Using Docker Command**
```bash
# On remote host
docker exec dc-airflow-apiserver-1 airflow variables set ENTSOE_API_TOKEN "your-token-here"
```

**Option C: Using Airflow UI**
1. Go to Admin → Variables
2. Add new variable:
   - Key: `ENTSOE_API_TOKEN`
   - Value: `your-token-here`

### Step 3: Verify Token is Set

```bash
# On remote host
docker exec dc-airflow-apiserver-1 airflow variables get ENTSOE_API_TOKEN
```

Expected output: Your token (first 20 characters shown)

### Step 4: Trigger DAG

**Option A: Using Airflow UI**
1. Go to DAGs → `entsoe_ingestion`
2. Click "Play" button to trigger manually

**Option B: Using CLI**
```bash
# On remote host
docker exec dc-airflow-apiserver-1 airflow dags trigger entsoe_ingestion
```

### Step 5: Verify Data Ingestion

**Check table has data:**
```bash
# On remote host
docker exec dc-airflow-apiserver-1 bash -c 'export PGPASSWORD=$POSTGRES_PASSWORD; psql -h 172.18.0.1 -U airflow -d lianel_energy -c "SELECT COUNT(*) FROM fact_electricity_timeseries;"'
```

**Check API endpoint:**
```bash
curl -s 'https://www.lianel.se/api/v1/electricity/timeseries?limit=1' | python3 -m json.tool
```

**Check ingestion log:**
```bash
# On remote host
docker exec dc-airflow-apiserver-1 bash -c 'export PGPASSWORD=$POSTGRES_PASSWORD; psql -h 172.18.0.1 -U airflow -d lianel_energy -c "SELECT country_code, records_ingested, status FROM meta_entsoe_ingestion_log ORDER BY ingestion_date DESC LIMIT 10;"'
```

## Expected Results

After setting the token and triggering the DAG:

1. ✅ **Table populated**: `fact_electricity_timeseries` should have records
2. ✅ **API returns data**: `/api/v1/electricity/timeseries` should return records
3. ✅ **Ingestion log populated**: `meta_entsoe_ingestion_log` should show successful ingestions
4. ✅ **Frontend shows data**: Electricity Timeseries page should display data

## Notes

- The DAG runs daily at 03:00 UTC (after ENTSO-E data is available)
- The DAG processes 23 European countries
- Data is ingested with checkpoint/resume logic (skips already-ingested data)
- The token should be kept secure and not committed to version control

## References

- ENTSO-E Transparency Platform: https://transparency.entsoe.eu/
- API Documentation: https://transparency.entsoe.eu/content/static_content/Static%20content/web%20api/Guide.html
- Setup Guide: `docs/ENTSOE-API-TOKEN-SETUP.md`
- Script: `scripts/set-entsoe-token.sh`
