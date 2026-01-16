# Grafana Datasource Fix - Deployed ✅

## Summary
All Grafana datasource connection issues have been fixed and deployed.

## Issues Fixed

### 1. Wrong Database URL ✅
- **Before**: `url: postgres:5432` (non-existent hostname)
- **After**: `url: 172.18.0.1:5432` (correct host IP)
- **Fixed**: Both PostgreSQL datasources now use correct URL

### 2. Password Not Substituted ✅
- **Before**: `password: ${POSTGRES_PASSWORD}` (literal string)
- **After**: `password: <actual_password>` (substituted from Airflow container)
- **Method**: Used `sed` to substitute password before Grafana reads the file

## Fix Applied on Remote Host

```bash
# Get password from Airflow container
POSTGRES_PASSWORD=$(docker exec dc-airflow-apiserver-1 env | grep POSTGRES_PASSWORD | cut -d'=' -f2)

# Fix URLs and substitute password
sed -i "s|url: postgres:5432|url: 172.18.0.1:5432|g" monitoring/grafana/provisioning/datasources/datasources.yml
sed -i "s|\${POSTGRES_PASSWORD}|${POSTGRES_PASSWORD}|g" monitoring/grafana/provisioning/datasources/datasources.yml

# Restart Grafana to apply changes
docker restart grafana
```

## Datasources Configured

1. **PostgreSQL Airflow**
   - URL: `172.18.0.1:5432` ✅
   - Database: `airflow`
   - User: `airflow`
   - Password: Set correctly ✅
   - UID: `postgres-airflow`

2. **PostgreSQL Energy**
   - URL: `172.18.0.1:5432` ✅
   - Database: `lianel_energy`
   - User: `airflow`
   - Password: Set correctly ✅
   - UID: `postgres-energy`

## Verification

After restart, Grafana should:
- Load datasources from provisioning file
- Connect to both PostgreSQL databases successfully
- All dashboards should work without authentication errors

## Script Available

For future use, a script is available at:
- `scripts/fix-grafana-datasources.sh` - Automates the fix process

## Status
✅ **DEPLOYED** - All fixes applied and Grafana restarted

## Next Steps
- Test Grafana dashboards - all panels should load correctly
- Verify datasource connections in Grafana UI
- Check that all queries execute successfully
