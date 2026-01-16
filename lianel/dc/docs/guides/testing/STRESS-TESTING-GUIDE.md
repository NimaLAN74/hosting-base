# Stress Testing Guide
**Last Updated**: January 15, 2026

---

## Overview

Stress testing determines the breaking points of the system by pushing it beyond normal operating conditions.

---

## Objectives

1. Identify system limits
2. Determine breaking points
3. Test resource exhaustion scenarios
4. Validate error handling under stress
5. Document recovery procedures

---

## Test Scenarios

### 1. High Concurrent Users

**Objective**: Test system behavior with maximum concurrent users

**Test Parameters**:
- **Concurrent Users**: 500, 1000, 2000
- **Duration**: 5 minutes per level
- **Ramp-up**: 100 users per 30 seconds

**Expected Behavior**:
- System should handle load gracefully
- Response times may increase but should remain acceptable
- Error rate should remain low (< 1%)

**Breaking Point**: When error rate exceeds 5% or response time exceeds 10s

**Procedure**:
```bash
# Using wrk
wrk -t16 -c500 -d5m --timeout 30s \
    -H "Authorization: Bearer $TOKEN" \
    https://www.lianel.se/api/v1/datasets/forecasting?limit=10

# Increase to 1000
wrk -t16 -c1000 -d5m --timeout 30s \
    -H "Authorization: Bearer $TOKEN" \
    https://www.lianel.se/api/v1/datasets/forecasting?limit=10

# Increase to 2000
wrk -t16 -c2000 -d5m --timeout 30s \
    -H "Authorization: Bearer $TOKEN" \
    https://www.lianel.se/api/v1/datasets/forecasting?limit=10
```

---

### 2. Database Connection Exhaustion

**Objective**: Test system behavior when database connections are exhausted

**Test Parameters**:
- **Concurrent Requests**: 200
- **Duration**: Until failure
- **Database Connection Pool**: Monitor pool size

**Expected Behavior**:
- System should queue requests when pool is exhausted
- Requests should timeout gracefully
- Error messages should be clear

**Breaking Point**: When all connections are exhausted and new requests fail

**Procedure**:
```bash
# Monitor database connections
watch -n 1 'psql -h localhost -U airflow -d airflow -c "SELECT count(*) FROM pg_stat_activity;"'

# Run stress test
wrk -t16 -c200 -d10m --timeout 30s \
    -H "Authorization: Bearer $TOKEN" \
    https://www.lianel.se/api/v1/datasets/forecasting?limit=1000
```

---

### 3. Memory Exhaustion

**Objective**: Test system behavior when memory is exhausted

**Test Parameters**:
- **Memory Limit**: Monitor container memory
- **Test Duration**: Until OOM kill
- **Concurrent Requests**: Gradual increase

**Expected Behavior**:
- System should handle memory pressure
- Containers should be killed gracefully
- Services should restart automatically

**Breaking Point**: When containers are OOM-killed

**Procedure**:
```bash
# Monitor memory usage
watch -n 1 'docker stats --no-stream'

# Run stress test with large payloads
wrk -t16 -c100 -d30m --timeout 30s \
    -H "Authorization: Bearer $TOKEN" \
    -s large-query.lua \
    https://www.lianel.se/api/v1/datasets/forecasting?limit=10000
```

---

### 4. CPU Exhaustion

**Objective**: Test system behavior when CPU is exhausted

**Test Parameters**:
- **CPU Usage**: Monitor CPU usage
- **Test Duration**: 10 minutes
- **Concurrent Requests**: High volume

**Expected Behavior**:
- System should handle CPU pressure
- Response times may increase
- Services should remain responsive

**Breaking Point**: When CPU usage is 100% and response times exceed 30s

**Procedure**:
```bash
# Monitor CPU usage
watch -n 1 'docker stats --no-stream | grep CPU'

# Run CPU-intensive stress test
wrk -t32 -c500 -d10m --timeout 30s \
    -H "Authorization: Bearer $TOKEN" \
    https://www.lianel.se/api/v1/datasets/forecasting?limit=1000
```

---

### 5. Disk Space Exhaustion

**Objective**: Test system behavior when disk space is exhausted

**Test Parameters**:
- **Disk Usage**: Monitor disk space
- **Test Duration**: Until disk is full
- **Test Method**: Generate large log files or data

**Expected Behavior**:
- System should handle disk pressure
- Services may fail when disk is full
- Error messages should be clear

**Breaking Point**: When disk is 100% full and services fail

**Procedure**:
```bash
# Monitor disk space
watch -n 1 'df -h'

# Fill disk (careful - test environment only!)
dd if=/dev/zero of=/tmp/testfile bs=1M count=10000

# Monitor service behavior
docker ps
docker logs <service-name>
```

---

### 6. Network Bandwidth Exhaustion

**Objective**: Test system behavior when network bandwidth is exhausted

**Test Parameters**:
- **Bandwidth**: Monitor network usage
- **Test Duration**: 10 minutes
- **Concurrent Requests**: High volume with large responses

**Expected Behavior**:
- System should handle bandwidth pressure
- Response times may increase
- Services should remain functional

**Breaking Point**: When bandwidth is saturated and response times exceed 30s

**Procedure**:
```bash
# Monitor network usage
watch -n 1 'iftop -i eth0'

# Run bandwidth-intensive stress test
wrk -t16 -c200 -d10m --timeout 30s \
    -H "Authorization: Bearer $TOKEN" \
    https://www.lianel.se/api/v1/datasets/forecasting?limit=10000
```

---

## Test Execution

### Pre-Test Checklist
- [ ] Backup system state
- [ ] Notify stakeholders
- [ ] Set up monitoring
- [ ] Prepare rollback plan
- [ ] Document baseline metrics

### During Test
- [ ] Monitor system metrics
- [ ] Record breaking points
- [ ] Document error messages
- [ ] Capture logs
- [ ] Note recovery behavior

### Post-Test
- [ ] Restore system state
- [ ] Analyze results
- [ ] Document findings
- [ ] Create recommendations
- [ ] Update capacity planning

---

## Metrics to Monitor

### System Metrics
- CPU usage per container
- Memory usage per container
- Disk I/O
- Network I/O
- Container restart count

### Application Metrics
- Request rate (RPS)
- Response time (p50, p95, p99)
- Error rate
- Timeout rate
- Throughput

### Database Metrics
- Connection pool usage
- Query execution time
- Lock contention
- Transaction rate

---

## Breaking Points Documentation

### Document for Each Test
1. **Test Scenario**: Description
2. **Breaking Point**: When system failed
3. **Symptoms**: What was observed
4. **Recovery Time**: How long to recover
5. **Recommendations**: How to prevent/improve

### Example Format
```
Test: High Concurrent Users
Breaking Point: 1500 concurrent users
Symptoms:
  - Error rate: 8%
  - Response time p95: 15s
  - Database connections: 100% utilized
Recovery Time: 2 minutes after load reduction
Recommendations:
  - Increase database connection pool
  - Add connection pooling at application level
  - Implement request queuing
```

---

## Safety Considerations

### Test Environment
- [ ] Use dedicated test environment
- [ ] Isolate from production
- [ ] Have rollback plan ready
- [ ] Monitor closely
- [ ] Stop test if critical issues occur

### Production Testing
- [ ] Schedule during low-traffic periods
- [ ] Start with low load
- [ ] Gradually increase load
- [ ] Have rollback plan ready
- [ ] Monitor closely
- [ ] Stop immediately if issues occur

---

## Tools

### Load Testing
- **wrk**: HTTP benchmarking tool
- **Apache Bench (ab)**: Simple HTTP benchmarking
- **JMeter**: Advanced load testing
- **k6**: Modern load testing tool

### Monitoring
- **Grafana**: Real-time metrics visualization
- **Prometheus**: Metrics collection
- **docker stats**: Container resource usage
- **htop/top**: System resource monitoring

---

## Success Criteria

- [ ] All breaking points identified
- [ ] Recovery procedures tested
- [ ] Recommendations documented
- [ ] Capacity limits defined
- [ ] Monitoring alerts configured

---

**Status**: Active  
**Last Review**: January 15, 2026
