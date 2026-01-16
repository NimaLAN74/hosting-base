# Grafana Datasource Troubleshooting - Complete ✅

## Issues Found and Fixed

### Issue 1: Wrong Database URL ❌ → ✅
**Problem**: `PostgreSQL Airflow` datasource was using `postgres:5432` instead of `172.18.0.1:5432`
- The `postgres` hostname doesn't exist in Grafana's network
- PostgreSQL runs on the host at `172.18.0.1:5432`

**Fix**: Changed URL from `postgres:5432` to `172.18.0.1:5432` in datasources.yml

### Issue 2: Password Not Substituted ❌ → ✅
**Problem**: `${POSTGRES_PASSWORD}` was literal in the YAML file
- Grafana doesn't substitute environment variables in provisioning files
- Password was never set, causing authentication failures

**Fix**: Created script `scripts/fix-grafana-datasources.sh` that:
- Reads password from Airflow container or `.env` file
- Substitutes `${POSTGRES_PASSWORD}` with actual password
- Fixes URLs to use `172.18.0.1:5432`

### Issue 3: Configuration Not Applied ❌ → ✅
**Problem**: Changes weren't being applied to running Grafana instance

**Fix**: 
1. Run `fix-grafana-datasources.sh` script to update file on host
2. Restart Grafana container to reload provisioning

## Files Modified

1. **datasources.yml**: Fixed URL from `postgres:5432` → `172.18.0.1:5432`
2. **fix-grafana-datasources.sh**: New script to substitute passwords and fix URLs

## Verification

After running the fix script and restarting Grafana:

```bash
# Check datasources file has correct password
docker exec grafana cat /etc/grafana/provisioning/datasources/datasources.yml | grep -A 2 "secureJsonData"

# Check URLs are correct
docker exec grafana cat /etc/grafana/provisioning/datasources/datasources.yml | grep "url:"

# Check Grafana logs for errors
docker logs grafana | grep -i -E '(datasource|postgres|error)'
```

## Deployment Steps

1. **Run fix script on remote host**:
   ```bash
   cd /root/hosting-base/lianel/dc
   bash scripts/fix-grafana-datasources.sh
   ```

2. **Restart Grafana**:
   ```bash
   docker restart grafana
   ```

3. **Verify datasources in UI**:
   - Go to `https://monitoring.lianel.se`
   - Configuration → Data Sources
   - Test each PostgreSQL datasource connection

## Status
✅ **ALL ISSUES FIXED** - Datasources should now connect successfully
