# DAG Failure Recovery Runbook
**Last Updated**: January 15, 2026  
**Owner**: Operations Team

---

## Overview

This runbook provides step-by-step procedures for recovering from Airflow DAG failures.

---

## Quick Reference

| Issue | Quick Fix | Full Procedure |
|-------|-----------|----------------|
| Single task failure | Retry task | Section 2.1 |
| DAG run failure | Clear and rerun | Section 2.2 |
| Multiple DAG failures | Check system health | Section 2.3 |
| Persistent failures | Investigate root cause | Section 3 |

---

## 1. Pre-Recovery Checks

### 1.1 Verify System Health
```bash
# Check Airflow services
docker ps | grep airflow

# Check Airflow scheduler
docker logs dc-airflow-scheduler-1 --tail 50

# Check Airflow worker
docker logs dc-airflow-worker-1 --tail 50

# Check system resources
free -h
df -h
```

### 1.2 Check Recent Failures
```bash
# View recent failed DAG runs
docker exec dc-airflow-scheduler-1 airflow dags list-runs -d <dag_id> --state failed --limit 10

# View failed tasks
docker exec dc-airflow-scheduler-1 airflow tasks list <dag_id> --state failed
```

---

## 2. Recovery Procedures

### 2.1 Single Task Failure

#### Symptoms
- One task in a DAG has failed
- Other tasks in the DAG are successful
- DAG run is in "failed" state

#### Recovery Steps

1. **Identify the failed task**:
   ```bash
   docker exec dc-airflow-scheduler-1 airflow tasks list <dag_id> --state failed
   ```

2. **Check task logs**:
   - Access Airflow UI: `https://airflow.lianel.se`
   - Navigate to DAG → Failed task → Logs
   - Review error message

3. **Retry the task**:
   ```bash
   # Via CLI
   docker exec dc-airflow-scheduler-1 airflow tasks run <dag_id> <task_id> <execution_date>
   
   # Via UI
   # Click "Clear" on the failed task, then "Trigger DAG"
   ```

4. **Monitor retry**:
   - Watch task status in Airflow UI
   - Check logs if it fails again

#### Common Causes
- Transient network errors
- Database connection timeouts
- External API rate limits
- Resource constraints (OOM)

---

### 2.2 DAG Run Failure

#### Symptoms
- Entire DAG run has failed
- Multiple tasks failed
- DAG marked as "failed"

#### Recovery Steps

1. **Identify failed tasks**:
   ```bash
   docker exec dc-airflow-scheduler-1 airflow tasks list <dag_id> --state failed
   ```

2. **Check DAG run logs**:
   - Review all failed task logs
   - Identify root cause
   - Check for common patterns

3. **Clear and rerun**:
   ```bash
   # Clear all failed tasks
   docker exec dc-airflow-scheduler-1 airflow dags clear <dag_id> --start-date <start> --end-date <end>
   
   # Trigger new run
   docker exec dc-airflow-scheduler-1 airflow dags trigger <dag_id>
   ```

4. **Alternative: Clear specific tasks**:
   ```bash
   # Clear only failed tasks
   docker exec dc-airflow-scheduler-1 airflow tasks clear <dag_id> --task-regex <pattern> --start-date <start> --end-date <end>
   ```

#### Common Causes
- Configuration errors
- Data quality issues
- External service outages
- Resource exhaustion

---

### 2.3 Multiple DAG Failures

#### Symptoms
- Multiple DAGs failing simultaneously
- System-wide issues
- High failure rate

#### Recovery Steps

1. **Check system health**:
   ```bash
   # Check memory
   free -h
   
   # Check disk space
   df -h
   
   # Check Airflow worker
   docker stats dc-airflow-worker-1
   
   # Check for OOM kills
   dmesg | grep -i oom
   ```

2. **Check Airflow services**:
   ```bash
   # Verify all services are running
   docker ps | grep airflow
   
   # Check scheduler health
   curl http://localhost:8974/health
   
   # Check worker health
   docker exec dc-airflow-worker-1 celery --app airflow.providers.celery.executors.celery_executor.app inspect ping
   ```

3. **Check database connectivity**:
   ```bash
   # Test PostgreSQL connection
   docker exec dc-airflow-scheduler-1 airflow db check
   ```

4. **Restart services if needed**:
   ```bash
   # Restart worker (if memory issues)
   docker restart dc-airflow-worker-1
   
   # Restart scheduler (if stuck)
   docker restart dc-airflow-scheduler-1
   ```

5. **Clear stuck tasks**:
   ```bash
   # Find stuck tasks
   docker exec dc-airflow-scheduler-1 airflow tasks list <dag_id> --state running
   
   # Clear stuck tasks
   docker exec dc-airflow-scheduler-1 airflow tasks clear <dag_id> --task-regex <pattern>
   ```

---

## 3. Root Cause Investigation

### 3.1 Check Logs

#### Airflow Logs
```bash
# Scheduler logs
docker logs dc-airflow-scheduler-1 --tail 100

# Worker logs
docker logs dc-airflow-worker-1 --tail 100

# Task-specific logs
# Access via Airflow UI or:
docker exec dc-airflow-worker-1 find /opt/airflow/logs -name "*<task_id>*" -type f | head -1 | xargs cat
```

#### System Logs
```bash
# Check for OOM kills
dmesg | grep -i "killed process"

# Check Docker logs
docker logs <container_name> --tail 100

# Check system logs
journalctl -u docker --tail 100
```

### 3.2 Check Metrics

#### Prometheus Metrics
- Check Grafana dashboards:
  - System Health: `https://monitoring.lianel.se/d/system-health`
  - Pipeline Status: `https://monitoring.lianel.se/d/pipeline-status`
  - Error Tracking: `https://monitoring.lianel.se/d/error-tracking`

#### Database Metrics
```bash
# Check active connections
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy -c "SELECT count(*) FROM pg_stat_activity;"

# Check long-running queries
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy -c "SELECT pid, now() - pg_stat_activity.query_start AS duration, query FROM pg_stat_activity WHERE state = 'active' AND now() - pg_stat_activity.query_start > interval '5 minutes';"
```

---

## 4. Common Failure Scenarios

### 4.1 OOM (Out of Memory) Kills

#### Symptoms
- Tasks killed with SIGKILL
- Error: "Process terminated by signal"
- High memory usage

#### Recovery
1. Check worker memory limits
2. Reduce worker concurrency if needed
3. Clear stuck tasks
4. Restart worker
5. Monitor memory usage

#### Prevention
- Monitor worker memory usage
- Set appropriate resource limits
- Reduce task parallelism if needed

---

### 4.2 Database Connection Failures

#### Symptoms
- "Connection refused" errors
- "Connection timeout" errors
- "Too many connections" errors

#### Recovery
1. Check PostgreSQL status
2. Check connection pool settings
3. Restart Airflow services
4. Check database connections

#### Prevention
- Monitor connection pool usage
- Set appropriate pool sizes
- Implement connection retries

---

### 4.3 External API Failures

#### Symptoms
- HTTP 429 (Rate Limited)
- HTTP 504 (Gateway Timeout)
- Connection timeouts

#### Recovery
1. Check external service status
2. Wait for rate limits to reset
3. Retry with exponential backoff
4. Consider alternative endpoints

#### Prevention
- Implement rate limiting
- Add retry logic with backoff
- Monitor API response times

---

### 4.4 Data Quality Issues

#### Symptoms
- Missing data errors
- Invalid data format errors
- Data validation failures

#### Recovery
1. Check data source status
2. Verify data format
3. Re-run data quality checks
4. Fix data issues if needed

#### Prevention
- Implement data validation
- Monitor data quality metrics
- Set up data quality alerts

---

## 5. Escalation

### When to Escalate
- Multiple DAGs failing for >1 hour
- System-wide outages
- Data loss or corruption
- Security incidents
- Unable to resolve after 30 minutes

### Escalation Path
1. **Level 1**: Operations team (try recovery procedures)
2. **Level 2**: Development team (if code/config issue)
3. **Level 3**: Infrastructure team (if system issue)
4. **Level 4**: Management (if business impact)

### Contact Information
- Operations: [Contact info]
- Development: [Contact info]
- On-call: [Contact info]

---

## 6. Post-Recovery

### 6.1 Verification
- Verify DAG runs successfully
- Check data quality
- Monitor for recurring issues
- Update monitoring if needed

### 6.2 Documentation
- Document root cause
- Update runbook if needed
- Create incident report
- Share lessons learned

### 6.3 Prevention
- Implement fixes for root cause
- Update monitoring/alerts
- Improve error handling
- Update documentation

---

## 7. Quick Commands Reference

```bash
# List failed DAGs
docker exec dc-airflow-scheduler-1 airflow dags list-runs --state failed

# Clear failed tasks
docker exec dc-airflow-scheduler-1 airflow tasks clear <dag_id> --start-date <date> --end-date <date>

# Trigger DAG
docker exec dc-airflow-scheduler-1 airflow dags trigger <dag_id>

# Check task status
docker exec dc-airflow-scheduler-1 airflow tasks list <dag_id>

# View task logs (via UI)
# https://airflow.lianel.se -> DAG -> Task -> Logs

# Restart worker
docker restart dc-airflow-worker-1

# Check worker health
docker exec dc-airflow-worker-1 celery --app airflow.providers.celery.executors.celery_executor.app inspect ping
```

---

**Status**: Active  
**Review Frequency**: Monthly  
**Last Review**: January 15, 2026
