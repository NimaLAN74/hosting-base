# Commands to Restart Containers on Remote Host

## Quick Restart (Grafana Only)
To restart just Grafana to apply dashboard and configuration changes:

```bash
cd /root/hosting-base/lianel/dc
docker-compose -f docker-compose.monitoring.yaml restart grafana
```

## Full Monitoring Stack Restart
To restart all monitoring containers:

```bash
cd /root/hosting-base/lianel/dc
docker-compose -f docker-compose.monitoring.yaml restart
```

## Using the Script
A convenience script is available:

```bash
cd /root/hosting-base/lianel/dc
bash scripts/restart-monitoring-containers.sh
```

This script will:
1. Pull latest changes from git
2. Restart Grafana
3. Check container status
4. Display next steps

## What Gets Applied
After restarting Grafana, the following changes will be active:
- ✅ Updated Data Quality dashboard queries (correct column names)
- ✅ Updated Grafana OAuth configuration (admin role mapping)
- ✅ Connections feature enabled
- ✅ Roles scope added to OAuth

## Verification
After restart, verify:
1. Grafana is running: `docker ps | grep grafana`
2. Logs are clean: `docker logs grafana --tail 20`
3. Dashboard loads: Navigate to Data Quality dashboard in Grafana
4. Admin access: Check if Configuration menu is visible
