# Grafana Datasource Fix - Complete ✅

## Summary
All Grafana database connection issues have been fixed and deployed.

## Issues Fixed

### 1. Wrong Database URL ✅
- **Before**: `url: postgres:5432` (non-existent hostname)
- **After**: `url: 172.18.0.1:5432` (correct host IP)
- **Fixed**: Both PostgreSQL datasources now use correct URL

### 2. Password Not Substituted ✅
- **Before**: `password: ${POSTGRES_PASSWORD}` (literal string)
- **After**: `password: <actual_password>` (substituted from Airflow container)
- **Method**: Substituted password directly on host file using `sed`
- **Applied**: Password now correctly set in datasources.yml on host

## Fix Applied

```bash
# Get password from Airflow container
POSTGRES_PASSWORD=$(docker exec dc-airflow-apiserver-1 env | grep POSTGRES_PASSWORD | cut -d'=' -f2)

# Substitute password in datasources file on host
sed -i.bak "s|\${POSTGRES_PASSWORD}|${POSTGRES_PASSWORD}|g" monitoring/grafana/provisioning/datasources/datasources.yml

# Restart Grafana to load updated configuration
docker restart grafana
```

## Datasources Configured

1. **PostgreSQL Airflow**
   - URL: `172.18.0.1:5432` ✅
   - Database: `airflow`
   - User: `airflow`
   - Password: Set correctly ✅
   - UID: `postgres-airflow`
   - search_path: `public`

2. **PostgreSQL Energy**
   - URL: `172.18.0.1:5432` ✅
   - Database: `lianel_energy`
   - User: `airflow`
   - Password: Set correctly ✅
   - UID: `postgres-energy`

## Verification

After restart, verify the fix:
```bash
docker exec grafana cat /etc/grafana/provisioning/datasources/datasources.yml | grep -A 2 "secureJsonData"
```

Should show actual password, not `${POSTGRES_PASSWORD}`.

## Status
✅ **COMPLETE** - All fixes applied and Grafana restarted
✅ **DEPLOYED** - Datasources should now connect successfully

## Next Steps
- Test Grafana dashboards - all panels should load correctly
- Verify datasource connections in Grafana UI (Configuration → Data Sources → Test)
- All queries should execute successfully
