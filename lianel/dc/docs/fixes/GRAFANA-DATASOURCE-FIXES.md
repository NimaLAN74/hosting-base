# Grafana Datasource Fixes - PostgreSQL Connection Issues

## Problem
Most Grafana charts showing errors due to PostgreSQL connection issues:
- Airflow user authentication failures
- PostgreSQL connection failures

## Root Causes

### 1. Datasource URL Issue
- **Remote host configuration**: Used `url: postgres:5432` (non-existent container)
- **Should be**: `url: 172.18.0.1:5432` (host PostgreSQL)
- **Impact**: Grafana couldn't connect to PostgreSQL database

### 2. Datasource Reference Format
- **Problem**: Some dashboards used string datasource names: `"datasource": "PostgreSQL"`
- **Should be**: UID object format: `{"type": "postgres", "uid": "postgres-airflow"}`
- **Impact**: Dashboards couldn't find the datasource correctly

### 3. Missing search_path
- **Problem**: PostgreSQL Airflow datasource missing `search_path: public`
- **Impact**: Queries might not find tables in the `public` schema

## Fixes Applied

### 1. Fixed Datasource Configuration
**File**: `monitoring/grafana/provisioning/datasources/datasources.yml`

**Changes**:
- Updated PostgreSQL Airflow datasource URL from `postgres:5432` to `172.18.0.1:5432`
- Added `search_path: public` to `jsonData`

```yaml
  - name: PostgreSQL Airflow
    type: postgres
    access: proxy
    url: 172.18.0.1:5432  # Changed from postgres:5432
    database: airflow
    user: airflow
    uid: postgres-airflow
    secureJsonData:
      password: ${POSTGRES_PASSWORD}
    jsonData:
      sslmode: disable
      maxOpenConns: 100
      maxIdleConns: 100
      connMaxLifetime: 14400
      postgresVersion: 1500
      timescaledb: false
      search_path: public  # Added
```

### 2. Fixed Dashboard Datasource References

#### database-performance.json
- Changed `"datasource": "PostgreSQL"` to `{"type": "postgres", "uid": "postgres-airflow"}`
- All 6 instances updated

#### energy-osm-features.json
- Changed `"datasource": "PostgreSQL Energy"` to `{"type": "postgres", "uid": "postgres-energy"}`
- All 4 instances updated

#### data-quality.json
- Changed `"datasource": "PostgreSQL Energy"` to `{"type": "postgres", "uid": "postgres-energy"}`
- All 8 instances updated

### 3. Verified Other Dashboards
- ✅ `pipeline-status.json` - Already using correct UID format
- ✅ `sla-monitoring.json` - Already using correct UID format
- ✅ `system-health.json` - Uses Prometheus (correct)
- ✅ `error-tracking.json` - Uses Loki (correct)
- ✅ `web-analytics.json` - Uses Loki (correct)
- ✅ `energy-metrics.json` - Uses PostgreSQL Energy with UID (correct)

## Verification

### Datasource Configuration
```bash
# Check datasource config
docker exec grafana cat /etc/grafana/provisioning/datasources/datasources.yml | grep -A 15 'PostgreSQL Airflow'
```

### Dashboard References
```bash
# Check for string datasource references (should be none)
grep -r '"datasource": "' monitoring/grafana/provisioning/dashboards/*.json | grep -v '{"type":'
```

## Deployment

1. **Update datasource configuration**:
   ```bash
   # Already updated in local file
   # Remote host updated via sed command
   ssh root@host "sed -i 's|url: postgres:5432|url: 172.18.0.1:5432|g' monitoring/grafana/provisioning/datasources/datasources.yml"
   ```

2. **Restart Grafana**:
   ```bash
   docker restart grafana
   ```

3. **Verify datasources**:
   - Go to Grafana → Configuration → Data Sources
   - Verify "PostgreSQL Airflow" and "PostgreSQL Energy" are configured correctly
   - Test connection for both datasources

## Expected Results

After fixes:
- ✅ All PostgreSQL queries should work
- ✅ No authentication errors
- ✅ No connection errors
- ✅ All dashboard panels should load data correctly

## Related Files
- `monitoring/grafana/provisioning/datasources/datasources.yml`
- `monitoring/grafana/provisioning/dashboards/database-performance.json`
- `monitoring/grafana/provisioning/dashboards/energy-osm-features.json`
- `monitoring/grafana/provisioning/dashboards/data-quality.json`
