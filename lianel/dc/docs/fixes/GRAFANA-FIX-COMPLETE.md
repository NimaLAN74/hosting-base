# Grafana Datasource Configuration - Complete ✅

## Summary
Grafana datasources have been configured via API on the remote host.

## Solution Applied

### Problem:
- Grafana doesn't substitute environment variables in provisioning YAML files
- `${POSTGRES_PASSWORD}` was not being replaced with actual password
- Datasources showed authentication errors

### Solution:
1. ✅ Created API-based configuration script: `scripts/configure-grafana-datasources.sh`
2. ✅ Script reads `POSTGRES_PASSWORD` from Airflow container environment
3. ✅ Script uses Grafana API to configure datasources with correct passwords

### Datasources Configured:
1. **PostgreSQL Airflow**
   - UID: `postgres-airflow`
   - Database: `airflow`
   - User: `airflow`
   - URL: `172.18.0.1:5432`

2. **PostgreSQL Energy**
   - UID: `postgres-energy`
   - Database: `lianel_energy`
   - User: `airflow`
   - URL: `172.18.0.1:5432`

## Configuration Method

The script:
1. Reads `POSTGRES_PASSWORD` from Airflow container environment
2. Reads Grafana admin credentials from `.env` file
3. Uses Grafana REST API to configure datasources
4. Sets passwords in `secureJsonData` field (encrypted storage)

## Verification

Run the following to verify datasources are configured:
```bash
cd /root/hosting-base/lianel/dc
GRAFANA_ADMIN_PASSWORD=$(cat .env | grep GRAFANA_ADMIN_PASSWORD | cut -d'=' -f2)
docker exec grafana curl -s -u admin:${GRAFANA_ADMIN_PASSWORD} \
  'http://localhost:3000/api/datasources' | \
  python3 -c 'import sys,json; ds=json.load(sys.stdin); \
  [print(f"{d.get(\"name\")} - {d.get(\"uid\")}") for d in ds if d.get("type")=="postgres"]'
```

## Status
✅ **COMPLETE** - All Grafana datasources configured with correct passwords

## Next Steps
- Test Grafana dashboards - all panels should load without authentication errors
- Verify queries work correctly in all dashboards
