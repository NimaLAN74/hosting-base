# Service Restart Procedures
**Last Updated**: January 15, 2026  
**Owner**: Operations Team

---

## Overview

This runbook provides procedures for restarting services in the Lianel platform.

---

## Quick Reference

| Service | Restart Command | Health Check |
|---------|----------------|--------------|
| Airflow Worker | `docker restart dc-airflow-worker-1` | Section 2.1 |
| Airflow Scheduler | `docker restart dc-airflow-scheduler-1` | Section 2.2 |
| Energy Service | `docker restart lianel-energy-service` | Section 2.3 |
| Profile Service | `docker restart lianel-profile-service` | Section 2.4 |
| Keycloak | `docker restart keycloak` | Section 2.5 |
| Nginx | `docker restart nginx-proxy` | Section 2.6 |
| All Services | `docker compose restart` | Section 3 |

---

## 1. Pre-Restart Checks

### 1.1 Verify Current Status
```bash
# Check all containers
docker ps --format 'table {{.Names}}\t{{.Status}}'

# Check specific service
docker ps | grep <service_name>

# Check service logs
docker logs <service_name> --tail 50
```

### 1.2 Check Dependencies
```bash
# Check database
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy -c "SELECT version();"

# Check Redis
docker exec dc-redis-1 redis-cli -a ${REDIS_PASSWORD} ping

# Check network
docker network ls | grep lianel-network
```

### 1.3 Check System Resources
```bash
# Memory
free -h

# Disk space
df -h

# CPU load
top -bn1 | head -10
```

---

## 2. Service-Specific Procedures

### 2.1 Airflow Worker

#### When to Restart
- OOM kills occurring
- Tasks stuck in "running" state
- High memory usage
- Worker unresponsive

#### Restart Procedure
```bash
# 1. Check current status
docker ps | grep airflow-worker
docker stats dc-airflow-worker-1 --no-stream

# 2. Check running tasks
docker exec dc-airflow-scheduler-1 airflow tasks list --state running

# 3. Restart worker
docker restart dc-airflow-worker-1

# 4. Wait for startup (30-60 seconds)
sleep 30

# 5. Verify health
docker exec dc-airflow-worker-1 celery --app airflow.providers.celery.executors.celery_executor.app inspect ping

# 6. Check logs
docker logs dc-airflow-worker-1 --tail 20
```

#### Health Check
- Worker responds to ping
- No errors in logs
- Memory usage normal
- Tasks can be scheduled

---

### 2.2 Airflow Scheduler

#### When to Restart
- Scheduler stuck
- DAGs not scheduling
- High CPU usage
- Scheduler errors

#### Restart Procedure
```bash
# 1. Check current status
docker ps | grep airflow-scheduler
docker logs dc-airflow-scheduler-1 --tail 50

# 2. Check DAG status
docker exec dc-airflow-scheduler-1 airflow dags list

# 3. Restart scheduler
docker restart dc-airflow-scheduler-1

# 4. Wait for startup (30-60 seconds)
sleep 30

# 5. Verify health
curl http://localhost:8974/health

# 6. Check logs
docker logs dc-airflow-scheduler-1 --tail 20
```

#### Health Check
- Health endpoint responds
- DAGs are being scheduled
- No errors in logs
- Scheduler process running

---

### 2.3 Energy Service

#### When to Restart
- Service unresponsive
- API errors
- High memory usage
- Database connection issues

#### Restart Procedure
```bash
# 1. Check current status
docker ps | grep energy-service
curl http://localhost:3001/health

# 2. Check logs
docker logs lianel-energy-service --tail 50

# 3. Restart service
docker restart lianel-energy-service

# 4. Wait for startup (10-20 seconds)
sleep 15

# 5. Verify health
curl http://localhost:3001/health

# 6. Check logs
docker logs lianel-energy-service --tail 20
```

#### Health Check
- Health endpoint responds
- Database connection established
- No errors in logs
- Service listening on port 3001

---

### 2.4 Profile Service

#### When to Restart
- Service unresponsive
- Authentication failures
- Keycloak connection issues

#### Restart Procedure
```bash
# 1. Check current status
docker ps | grep profile-service
curl http://localhost:3000/health

# 2. Check logs
docker logs lianel-profile-service --tail 50

# 3. Restart service
docker restart lianel-profile-service

# 4. Wait for startup (10-20 seconds)
sleep 15

# 5. Verify health
curl http://localhost:3000/health

# 6. Check logs
docker logs lianel-profile-service --tail 20
```

#### Health Check
- Health endpoint responds
- Keycloak connection working
- No errors in logs
- Service listening on port 3000

---

### 2.5 Keycloak

#### When to Restart
- Authentication failures
- Service unresponsive
- Database connection issues
- High memory usage

#### Restart Procedure
```bash
# 1. Check current status
docker ps | grep keycloak
curl -k https://auth.lianel.se/realms/lianel/.well-known/openid-configuration

# 2. Check logs
docker logs keycloak --tail 50

# 3. Restart service
docker restart keycloak

# 4. Wait for startup (60-120 seconds - Keycloak takes longer)
sleep 90

# 5. Verify health
curl -k https://auth.lianel.se/realms/lianel/.well-known/openid-configuration

# 6. Check logs
docker logs keycloak --tail 20
```

#### Health Check
- OIDC endpoint accessible
- Database connection established
- No errors in logs
- Service listening on port 8080

---

### 2.6 Nginx

#### When to Restart
- Routing issues
- SSL certificate updates
- Configuration changes
- Service unresponsive

#### Restart Procedure
```bash
# 1. Check current status
docker ps | grep nginx
curl -I http://localhost:80

# 2. Check logs
docker logs nginx-proxy --tail 50

# 3. Test configuration
docker exec nginx-proxy nginx -t

# 4. Restart service
docker restart nginx-proxy

# 5. Wait for startup (5-10 seconds)
sleep 5

# 6. Verify health
curl -I http://localhost:80
curl -I https://localhost:443

# 7. Check logs
docker logs nginx-proxy --tail 20
```

#### Health Check
- HTTP/HTTPS endpoints respond
- Configuration valid
- No errors in logs
- SSL certificates loaded

---

## 3. Full System Restart

### When to Use
- System-wide issues
- After infrastructure changes
- Planned maintenance
- Recovery from major outage

### Restart Procedure

#### 3.1 Graceful Shutdown
```bash
# 1. Stop DAGs (pause them)
docker exec dc-airflow-scheduler-1 airflow dags pause <dag_id>

# 2. Wait for running tasks to complete (optional)
# Monitor: https://airflow.lianel.se

# 3. Stop services in order
cd /root/lianel/dc

# Stop application services first
docker compose -f docker-compose.yaml stop

# Stop Airflow
docker compose -f docker-compose.airflow.yaml stop

# Stop infrastructure
docker compose -f docker-compose.infra.yaml stop

# Stop monitoring (optional - can keep running)
# docker compose -f docker-compose.monitoring.yaml stop
```

#### 3.2 System Checks
```bash
# Check system resources
free -h
df -h

# Check for stuck processes
ps aux | grep -E 'airflow|python|java'

# Clean up if needed
docker system prune -f
```

#### 3.3 Start Services
```bash
cd /root/lianel/dc

# Start infrastructure first
docker compose -f docker-compose.infra.yaml up -d

# Wait for Keycloak and database
sleep 30

# Start Airflow
docker compose -f docker-compose.airflow.yaml up -d

# Wait for Airflow
sleep 30

# Start application services
docker compose -f docker-compose.yaml up -d

# Start monitoring (if stopped)
# docker compose -f docker-compose.monitoring.yaml up -d
```

#### 3.4 Verification
```bash
# Check all services
docker ps --format 'table {{.Names}}\t{{.Status}}'

# Check service health
curl http://localhost:3001/health  # Energy service
curl http://localhost:3000/health  # Profile service
curl -k https://auth.lianel.se/realms/lianel/.well-known/openid-configuration  # Keycloak
curl http://localhost:8974/health  # Airflow scheduler

# Check frontend
curl -I https://lianel.se

# Unpause DAGs
docker exec dc-airflow-scheduler-1 airflow dags unpause <dag_id>
```

---

## 4. Emergency Procedures

### 4.1 Force Restart (If Graceful Fails)
```bash
# Force stop
docker stop $(docker ps -q)

# Force remove if needed
# docker rm -f $(docker ps -aq)

# Start services
cd /root/lianel/dc
docker compose -f docker-compose.infra.yaml up -d
sleep 30
docker compose -f docker-compose.airflow.yaml up -d
sleep 30
docker compose -f docker-compose.yaml up -d
```

### 4.2 Database Recovery
```bash
# Check PostgreSQL status
systemctl status postgresql

# Restart PostgreSQL (if on host)
systemctl restart postgresql

# Verify connection
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy -c "SELECT version();"
```

---

## 5. Post-Restart Verification

### 5.1 Service Health
- All containers running
- Health endpoints responding
- No errors in logs
- Services accessible

### 5.2 Functionality Tests
- Frontend loads: `https://lianel.se`
- Authentication works: `https://auth.lianel.se`
- API endpoints respond: `https://lianel.se/api/v1/...`
- Airflow UI accessible: `https://airflow.lianel.se`

### 5.3 Monitoring
- Check Grafana dashboards
- Verify alerts are working
- Check for any new errors
- Monitor resource usage

---

## 6. Troubleshooting

### Service Won't Start
1. Check logs: `docker logs <service>`
2. Check resources: `free -h`, `df -h`
3. Check dependencies: Database, Redis, network
4. Check configuration: `.env` file, compose files

### Service Starts But Unhealthy
1. Check health endpoint
2. Review logs for errors
3. Check dependencies
4. Verify configuration

### Intermittent Issues
1. Check system resources
2. Review monitoring dashboards
3. Check for OOM kills
4. Review error logs

---

## 7. Quick Commands Reference

```bash
# Restart single service
docker restart <service_name>

# Restart all services in compose file
docker compose -f <compose-file> restart

# Check service status
docker ps | grep <service>

# View service logs
docker logs <service> --tail 50

# Check service health
curl http://localhost:<port>/health

# Restart all (careful!)
cd /root/lianel/dc
docker compose -f docker-compose.infra.yaml restart
docker compose -f docker-compose.airflow.yaml restart
docker compose -f docker-compose.yaml restart
```

---

**Status**: Active  
**Review Frequency**: Quarterly  
**Last Review**: January 15, 2026
