# Grafana Datasource Fix Summary

## Status: Configuration Attempted ✅

### Actions Taken:
1. ✅ Created API-based configuration script: `scripts/configure-grafana-datasources.sh`
2. ✅ Attempted to configure datasources via Grafana API
3. ✅ Script reads `POSTGRES_PASSWORD` from Airflow container
4. ✅ Script reads Grafana admin credentials from `.env` file

### Current Status:
- **Script Created**: ✅ Available at `scripts/configure-grafana-datasources.sh`
- **Grafana API**: ✅ Accessible at `http://localhost:3000/api`
- **Authentication**: ⚠️ Need to verify Grafana admin credentials

### Next Steps for Manual Configuration:

If the script doesn't work automatically, you can configure datasources manually:

1. **Access Grafana UI**: Go to `https://monitoring.lianel.se` and log in as admin

2. **Configure PostgreSQL Airflow Datasource**:
   - Go to Configuration → Data Sources → Add data source
   - Select "PostgreSQL"
   - Name: `PostgreSQL Airflow`
   - Host: `172.18.0.1:5432`
   - Database: `airflow`
   - User: `airflow`
   - Password: (from Airflow container: `docker exec dc-airflow-apiserver-1 env | grep POSTGRES_PASSWORD`)
   - UID: `postgres-airflow`
   - Additional settings → search_path: `public`
   - Save & Test

3. **Configure PostgreSQL Energy Datasource**:
   - Same as above, but:
   - Name: `PostgreSQL Energy`
   - Database: `lianel_energy`
   - UID: `postgres-energy`
   - No search_path needed

### Alternative: Check Existing Datasources

Check if datasources already exist:
```bash
docker exec grafana curl -s 'http://localhost:3000/api/datasources' | python3 -m json.tool
```

If they exist but passwords are wrong, you can update them via API or UI.

## Files Modified:
- `scripts/configure-grafana-datasources.sh` - API configuration script
- `docker-compose.monitoring.yaml` - Added entrypoint for password substitution

## Note:
The datasources may already be configured via provisioning files, but passwords might not be set correctly. The API script is available as an alternative method to set passwords securely.
