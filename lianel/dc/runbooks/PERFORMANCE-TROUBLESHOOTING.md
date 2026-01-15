# Performance Troubleshooting Guide
**Last Updated**: January 15, 2026  
**Owner**: Operations Team

---

## Overview

This guide provides procedures for identifying and resolving performance issues in the Lianel platform.

---

## Quick Reference

| Issue | Symptoms | Quick Check |
|-------|----------|-------------|
| High CPU | CPU >80% | `top`, `docker stats` |
| High Memory | Memory >90% | `free -h`, `docker stats` |
| Slow Queries | Query >5s | Database logs, Grafana |
| Slow API | Response >500ms | API logs, Prometheus |
| OOM Kills | SIGKILL errors | `dmesg`, container logs |

---

## 1. System Performance Issues

### 1.1 High CPU Usage

#### Symptoms
- CPU usage >80%
- Slow system response
- High load average

#### Investigation
```bash
# Check system CPU
top -bn1 | head -20

# Check per-container CPU
docker stats --no-stream

# Check load average
uptime

# Check specific container
docker stats <container_name> --no-stream
```

#### Resolution
1. **Identify high CPU containers**:
   ```bash
   docker stats --no-stream --format "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}"
   ```

2. **Check container processes**:
   ```bash
   docker top <container_name>
   ```

3. **Check for runaway processes**:
   - Review container logs
   - Check for infinite loops
   - Look for resource leaks

4. **Optimize or restart**:
   - Restart container if needed
   - Reduce resource limits
   - Optimize application code

---

### 1.2 High Memory Usage

#### Symptoms
- Memory usage >90%
- OOM kills occurring
- Swap usage high

#### Investigation
```bash
# Check system memory
free -h

# Check per-container memory
docker stats --no-stream

# Check for OOM kills
dmesg | grep -i oom
journalctl -k | grep -i "out of memory"

# Check swap
swapon --show
```

#### Resolution
1. **Identify high memory containers**:
   ```bash
   docker stats --no-stream --format "table {{.Name}}\t{{.MemUsage}}\t{{.MemPerc}}"
   ```

2. **Check memory limits**:
   ```bash
   docker inspect <container_name> | grep -A 5 Memory
   ```

3. **Reduce memory usage**:
   - Restart containers
   - Reduce concurrency/parallelism
   - Optimize application memory usage
   - Increase container limits if appropriate

4. **For Airflow**:
   ```bash
   # Reduce worker concurrency
   # Edit docker-compose.airflow.yaml
   # AIRFLOW__CELERY__WORKER_CONCURRENCY: 2
   docker compose -f docker-compose.airflow.yaml restart airflow-worker
   ```

---

### 1.3 Disk I/O Issues

#### Symptoms
- Slow disk operations
- High I/O wait
- Disk space low

#### Investigation
```bash
# Check disk space
df -h

# Check disk I/O
iostat -x 1 5

# Check I/O wait
top -bn1 | grep "wa"

# Check Docker disk usage
docker system df
```

#### Resolution
1. **Free disk space**:
   ```bash
   # Clean Docker
   docker system prune -f
   
   # Remove old logs
   find /var/log -name "*.log" -mtime +7 -delete
   
   # Remove old backups
   find /root/backups -name "*.gz" -mtime +30 -delete
   ```

2. **Optimize I/O**:
   - Move data to faster storage
   - Optimize database queries
   - Reduce logging verbosity

---

## 2. Database Performance Issues

### 2.1 Slow Queries

#### Symptoms
- Queries taking >5 seconds
- Database CPU high
- Connection timeouts

#### Investigation
```bash
# Check active queries
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
SELECT 
    pid,
    now() - query_start AS duration,
    state,
    query
FROM pg_stat_activity
WHERE state = 'active'
  AND now() - query_start > interval '5 seconds'
ORDER BY duration DESC;
EOF

# Check sequential scans
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
SELECT 
    schemaname,
    tablename,
    seq_scan,
    idx_scan,
    seq_tup_read,
    idx_tup_fetch
FROM pg_stat_user_tables
WHERE seq_scan > 100
ORDER BY seq_scan DESC;
EOF
```

#### Resolution
1. **Identify slow queries**:
   - Review query execution plans
   - Check for missing indexes
   - Look for full table scans

2. **Add indexes**:
   ```bash
   # Run performance analysis
   bash scripts/analyze-db-performance.sh
   
   # Add indexes via migration
   ```

3. **Optimize queries**:
   - Rewrite inefficient queries
   - Add WHERE clauses
   - Use appropriate JOINs

4. **Kill long-running queries** (if needed):
   ```bash
   PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
   SELECT pg_terminate_backend(pid)
   FROM pg_stat_activity
   WHERE pid = <process_id>;
   EOF
   ```

---

### 2.2 High Connection Count

#### Symptoms
- "Too many connections" errors
- Database slow to respond
- Connection pool exhausted

#### Investigation
```bash
# Check connections
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
SELECT 
    count(*) as total_connections,
    count(*) FILTER (WHERE state = 'active') as active,
    count(*) FILTER (WHERE state = 'idle') as idle,
    count(*) FILTER (WHERE state = 'idle in transaction') as idle_in_transaction
FROM pg_stat_activity
WHERE datname = 'lianel_energy';

SELECT 
    application_name,
    count(*) as connections
FROM pg_stat_activity
WHERE datname = 'lianel_energy'
GROUP BY application_name;
EOF
```

#### Resolution
1. **Check connection limits**:
   ```bash
   PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy -c "SHOW max_connections;"
   ```

2. **Kill idle connections**:
   ```bash
   PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
   SELECT pg_terminate_backend(pid)
   FROM pg_stat_activity
   WHERE datname = 'lianel_energy'
     AND state = 'idle'
     AND state_change < now() - interval '10 minutes';
   EOF
   ```

3. **Adjust connection pool**:
   - Reduce pool size in applications
   - Implement connection pooling
   - Use connection timeouts

---

## 3. Application Performance Issues

### 3.1 Slow API Responses

#### Symptoms
- API response time >500ms
- Timeout errors
- High latency

#### Investigation
```bash
# Check API logs
docker logs lianel-energy-service --tail 100 | grep -E "duration|timeout|slow"

# Check Prometheus metrics
curl http://localhost:9090/api/v1/query?query=histogram_quantile\(0.95,rate\(http_request_duration_seconds_bucket\[5m\]\)\)

# Check Grafana dashboard
# https://monitoring.lianel.se/d/sla-monitoring
```

#### Resolution
1. **Identify slow endpoints**:
   - Review API logs
   - Check Prometheus metrics
   - Use Grafana dashboards

2. **Optimize queries**:
   - Add database indexes
   - Optimize SQL queries
   - Add caching

3. **Scale if needed**:
   - Increase container resources
   - Add more instances
   - Use load balancing

---

### 3.2 Airflow Performance Issues

#### Symptoms
- DAGs taking too long
- Tasks stuck in queue
- High task execution time

#### Investigation
```bash
# Check task queue
docker exec dc-airflow-scheduler-1 airflow tasks list --state queued

# Check worker status
docker exec dc-airflow-worker-1 celery --app airflow.providers.celery.executors.celery_executor.app inspect active

# Check DAG duration
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
SELECT 
    dag_id,
    AVG(EXTRACT(EPOCH FROM (end_date - start_date))) as avg_duration_seconds
FROM dag_run
WHERE state = 'success'
  AND end_date > NOW() - INTERVAL '7 days'
GROUP BY dag_id
ORDER BY avg_duration_seconds DESC;
EOF
```

#### Resolution
1. **Optimize DAGs**:
   - Reduce task parallelism
   - Optimize task dependencies
   - Use TaskGroups efficiently

2. **Adjust Airflow config**:
   ```yaml
   # Reduce parallelism
   AIRFLOW__CORE__PARALLELISM: 4
   AIRFLOW__CELERY__WORKER_CONCURRENCY: 2
   ```

3. **Optimize tasks**:
   - Optimize Python code
   - Use batch processing
   - Add caching where appropriate

---

## 4. Network Performance Issues

### 4.1 Slow Network I/O

#### Symptoms
- Slow API calls
- Timeout errors
- High latency

#### Investigation
```bash
# Check network I/O
ifstat -i eth0 1 5

# Check network connections
netstat -an | grep ESTABLISHED | wc -l

# Test connectivity
ping -c 5 8.8.8.8
curl -w "@-" -o /dev/null -s http://example.com <<'EOF'
     time_namelookup:  %{time_namelookup}\n
        time_connect:  %{time_connect}\n
     time_appconnect:  %{time_appconnect}\n
    time_pretransfer:  %{time_pretransfer}\n
       time_redirect:  %{time_redirect}\n
  time_starttransfer:  %{time_starttransfer}\n
                     ----------\n
          time_total:  %{time_total}\n
EOF
```

#### Resolution
1. **Check network configuration**:
   - Verify DNS settings
   - Check firewall rules
   - Review network limits

2. **Optimize network usage**:
   - Use connection pooling
   - Implement caching
   - Reduce API calls

---

## 5. Performance Monitoring

### 5.1 Grafana Dashboards
- System Health: `https://monitoring.lianel.se/d/system-health`
- Pipeline Status: `https://monitoring.lianel.se/d/pipeline-status`
- SLA Monitoring: `https://monitoring.lianel.se/d/sla-monitoring`

### 5.2 Prometheus Queries
```promql
# CPU usage
100 - (avg(rate(node_cpu_seconds_total{mode="idle"}[5m])) * 100)

# Memory usage
(1 - (node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes)) * 100

# API response time
histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))
```

---

## 6. Performance Optimization

### 6.1 Database Optimization
- Regular VACUUM ANALYZE
- Add appropriate indexes
- Optimize queries
- Monitor connection pool

### 6.2 Application Optimization
- Optimize code
- Add caching
- Use async operations
- Implement pagination

### 6.3 Infrastructure Optimization
- Right-size containers
- Use resource limits
- Optimize Docker configuration
- Monitor resource usage

---

## 7. Quick Commands Reference

```bash
# Check system resources
free -h
df -h
top -bn1 | head -20

# Check container resources
docker stats --no-stream

# Check for OOM kills
dmesg | grep -i oom

# Check slow queries
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy -c "SELECT pid, now() - query_start AS duration, query FROM pg_stat_activity WHERE state = 'active' AND now() - query_start > interval '5 seconds';"

# Check API performance
curl http://localhost:9090/api/v1/query?query=histogram_quantile\(0.95,rate\(http_request_duration_seconds_bucket\[5m\]\)\)

# Restart high-resource container
docker restart <container_name>
```

---

**Status**: Active  
**Review Frequency**: Quarterly  
**Last Review**: January 15, 2026
