# Grafana Datasource Fix - Final Deployment ✅

## Issues Fixed

### 1. Wrong Database URL ✅
- Changed from: `postgres:5432` → `172.18.0.1:5432`
- Applied to both PostgreSQL datasources

### 2. Password Substitution ✅
- **Solution**: Use entrypoint script to substitute password on container startup
- **Method**: Entrypoint runs `sed` before Grafana starts
- **Volume**: Made provisioning volume writable (not `:ro`) so entrypoint can modify file

## Configuration Changes

### docker-compose.monitoring.yaml
- Removed `:ro` from provisioning volume mount
- Added entrypoint that substitutes `POSTGRES_PASSWORD` before Grafana starts

### Entrypoint Script
```bash
sh -c "
  if [ -n \"$$POSTGRES_PASSWORD\" ]; then
    sed -i.bak \"s|\\\$${POSTGRES_PASSWORD}|$$POSTGRES_PASSWORD|g\" /etc/grafana/provisioning/datasources/datasources.yml
  fi
  /run.sh
"
```

## Deployment Status

✅ **Deployed** - Grafana restarted with:
- Correct database URLs (`172.18.0.1:5432`)
- Password substitution via entrypoint
- Writable provisioning volume

## Verification

Check that passwords were substituted:
```bash
docker exec grafana cat /etc/grafana/provisioning/datasources/datasources.yml | grep -A 2 "secureJsonData"
```

Should show actual password, not `${POSTGRES_PASSWORD}`.

## Status
✅ **COMPLETE** - All fixes deployed and Grafana restarted
