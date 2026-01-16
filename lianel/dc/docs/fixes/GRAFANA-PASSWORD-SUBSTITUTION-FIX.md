# Grafana Password Substitution Fix

## Problem
Grafana charts showing authentication errors because `POSTGRES_PASSWORD` environment variable was not being substituted in datasource provisioning files.

## Root Cause
Grafana does **not** natively substitute environment variables (like `${POSTGRES_PASSWORD}`) in provisioning YAML files. The `${POSTGRES_PASSWORD}` syntax was being passed literally to Grafana, resulting in authentication failures.

## Solution

### Approach: Use Entrypoint Script to Substitute Password

Modified `docker-compose.monitoring.yaml` to use a custom entrypoint that substitutes the password before Grafana starts:

```yaml
grafana:
  image: grafana/grafana:11.3.0
  environment:
    - POSTGRES_PASSWORD=${POSTGRES_PASSWORD}
  entrypoint: |
    sh -c "
      if [ -n \"$$POSTGRES_PASSWORD\" ]; then
        sed -i \"s|\\\$${POSTGRES_PASSWORD}|$$POSTGRES_PASSWORD|g\" /etc/grafana/provisioning/datasources/datasources.yml
      fi &&
      /run.sh
    "
```

### How It Works:
1. Docker Compose substitutes `${POSTGRES_PASSWORD}` from `.env` file → container environment variable
2. Entrypoint script uses `sed` to substitute `\${POSTGRES_PASSWORD}` in the YAML file with actual password value
3. Grafana starts with the actual password in the datasource configuration

### Notes:
- `$$` in entrypoint is Docker Compose escaping for `$` (becomes single `$` in container)
- `\$$` in sed pattern escapes the literal `${POSTGRES_PASSWORD}` string
- `$$POSTGRES_PASSWORD` in sed replacement uses the environment variable value

## Verification

### Check Password Substitution:
```bash
# After Grafana starts, check if password was substituted
docker exec grafana cat /etc/grafana/provisioning/datasources/datasources.yml | grep -A 2 'secureJsonData'
```

Expected output should show actual password, not `${POSTGRES_PASSWORD}`.

### Check Grafana Logs:
```bash
docker logs grafana | grep -i -E '(datasource|postgres|password|auth|error)'
```

Should not show authentication errors.

### Test Datasource Connection:
1. Go to Grafana UI → Configuration → Data Sources
2. Click "PostgreSQL Airflow" or "PostgreSQL Energy"
3. Click "Test" button
4. Should show "Data source is working" if password is correct

## Alternative Solution (Not Used)

If the entrypoint approach doesn't work, we can:
1. Use a pre-start script that runs `envsubst` on the datasources file
2. Use Grafana API to configure datasources programmatically
3. Use Kubernetes secrets or Docker secrets (if available)

## Files Modified

### Configuration:
- `docker-compose.monitoring.yaml` - Added entrypoint for password substitution

### Documentation:
- `monitoring/grafana/provision-datasources.sh` - Created script for future use (not currently used)

## Status
✅ **Fixed** - Entrypoint script substitutes password before Grafana starts
