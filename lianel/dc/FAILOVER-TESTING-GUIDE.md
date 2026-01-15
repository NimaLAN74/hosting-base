# Failover Testing Guide
**Last Updated**: January 15, 2026

---

## Overview

Failover testing validates that the system can recover from service failures and continue operating with minimal disruption.

---

## Objectives

1. Test service restart scenarios
2. Test database failover
3. Test network partition scenarios
4. Test container restart scenarios
5. Validate recovery procedures
6. Measure recovery time objectives (RTO)

---

## Test Scenarios

### 1. Service Restart

#### 1.1 Single Service Restart

**Objective**: Test system behavior when a single service is restarted

**Test Procedure**:
```bash
# 1. Monitor service status
watch -n 1 'docker ps | grep <service-name>'

# 2. Generate load
wrk -t4 -c50 -d5m \
    -H "Authorization: Bearer $TOKEN" \
    https://www.lianel.se/api/v1/datasets/forecasting?limit=10

# 3. Restart service
docker restart <service-name>

# 4. Monitor recovery
# - Check when service becomes healthy
# - Check error rate during restart
# - Check request handling during restart
```

**Expected Behavior**:
- Service should restart within 30 seconds
- Requests should queue or fail gracefully
- Service should resume normal operation after restart
- No data loss

**Success Criteria**:
- [ ] Service restarts successfully
- [ ] Recovery time < 30 seconds
- [ ] Error rate during restart < 5%
- [ ] No data corruption

---

#### 1.2 Multiple Service Restart

**Objective**: Test system behavior when multiple services are restarted

**Test Procedure**:
```bash
# Restart multiple services simultaneously
docker restart lianel-energy-service lianel-profile-service

# Monitor system behavior
watch -n 1 'docker ps'
```

**Expected Behavior**:
- Services should restart independently
- System should handle partial availability
- Services should recover without intervention

**Success Criteria**:
- [ ] All services restart successfully
- [ ] System remains partially functional
- [ ] Services recover independently

---

### 2. Database Failover

#### 2.1 Database Connection Loss

**Objective**: Test system behavior when database connection is lost

**Test Procedure**:
```bash
# 1. Monitor database connections
watch -n 1 'psql -h localhost -U airflow -d airflow -c "SELECT count(*) FROM pg_stat_activity;"'

# 2. Generate load
wrk -t4 -c50 -d5m \
    -H "Authorization: Bearer $TOKEN" \
    https://www.lianel.se/api/v1/datasets/forecasting?limit=10

# 3. Stop database
sudo systemctl stop postgresql

# 4. Monitor error handling
# - Check error messages
# - Check retry behavior
# - Check connection pooling

# 5. Restart database
sudo systemctl start postgresql

# 6. Monitor recovery
# - Check when connections are restored
# - Check if requests resume
```

**Expected Behavior**:
- Requests should fail gracefully with clear error messages
- Connection pooling should handle reconnection
- Services should retry database connections
- System should recover when database is restored

**Success Criteria**:
- [ ] Error messages are clear
- [ ] Connection retry works
- [ ] System recovers when database is restored
- [ ] Recovery time < 2 minutes

---

#### 2.2 Database Restart

**Objective**: Test system behavior when database is restarted

**Test Procedure**:
```bash
# 1. Monitor active connections
psql -h localhost -U airflow -d airflow -c "SELECT count(*) FROM pg_stat_activity;"

# 2. Restart database
sudo systemctl restart postgresql

# 3. Monitor recovery
# - Check connection restoration
# - Check query execution
# - Check transaction recovery
```

**Expected Behavior**:
- Active connections should be closed gracefully
- Pending transactions should be rolled back
- New connections should be established after restart
- Services should reconnect automatically

**Success Criteria**:
- [ ] Database restarts successfully
- [ ] Connections are restored
- [ ] No data corruption
- [ ] Recovery time < 1 minute

---

### 3. Network Partition

#### 3.1 Service-to-Database Network Partition

**Objective**: Test system behavior when service cannot reach database

**Test Procedure**:
```bash
# 1. Block database port from service
sudo iptables -A DOCKER-USER -s <service-ip> -d <db-ip> -p tcp --dport 5432 -j DROP

# 2. Monitor service behavior
docker logs -f <service-name>

# 3. Generate load
wrk -t4 -c50 -d2m \
    -H "Authorization: Bearer $TOKEN" \
    https://www.lianel.se/api/v1/datasets/forecasting?limit=10

# 4. Restore network
sudo iptables -D DOCKER-USER -s <service-ip> -d <db-ip> -p tcp --dport 5432 -j DROP

# 5. Monitor recovery
```

**Expected Behavior**:
- Requests should fail with database connection errors
- Services should retry connections
- System should recover when network is restored

**Success Criteria**:
- [ ] Error handling is graceful
- [ ] Connection retry works
- [ ] System recovers when network is restored

---

#### 3.2 Service-to-Service Network Partition

**Objective**: Test system behavior when services cannot communicate

**Test Procedure**:
```bash
# 1. Block communication between services
sudo iptables -A DOCKER-USER -s <service1-ip> -d <service2-ip> -j DROP

# 2. Monitor service behavior
docker logs -f <service-name>

# 3. Restore network
sudo iptables -D DOCKER-USER -s <service1-ip> -d <service2-ip> -j DROP
```

**Expected Behavior**:
- Services should handle communication failures
- Error messages should be clear
- System should recover when network is restored

**Success Criteria**:
- [ ] Error handling is graceful
- [ ] System recovers when network is restored

---

### 4. Container Restart

#### 4.1 Container Crash

**Objective**: Test system behavior when container crashes

**Test Procedure**:
```bash
# 1. Force kill container
docker kill <container-name>

# 2. Monitor restart
watch -n 1 'docker ps -a | grep <container-name>'

# 3. Check restart policy
docker inspect <container-name> | grep RestartPolicy
```

**Expected Behavior**:
- Container should restart automatically (if restart policy is set)
- Service should recover after restart
- No data loss

**Success Criteria**:
- [ ] Container restarts automatically
- [ ] Service recovers successfully
- [ ] Recovery time < 1 minute

---

#### 4.2 Container OOM Kill

**Objective**: Test system behavior when container is OOM-killed

**Test Procedure**:
```bash
# 1. Monitor memory usage
watch -n 1 'docker stats --no-stream | grep <container-name>'

# 2. Generate memory pressure
# (Use stress test or memory-intensive operation)

# 3. Monitor OOM kill
dmesg | grep -i "oom\|killed"

# 4. Monitor restart
watch -n 1 'docker ps -a | grep <container-name>'
```

**Expected Behavior**:
- Container should be killed by kernel
- Container should restart automatically
- Service should recover after restart

**Success Criteria**:
- [ ] Container restarts automatically
- [ ] Service recovers successfully
- [ ] Memory limits prevent system-wide issues

---

### 5. Nginx Failover

#### 5.1 Nginx Restart

**Objective**: Test system behavior when Nginx is restarted

**Test Procedure**:
```bash
# 1. Generate load
wrk -t4 -c100 -d5m \
    -H "Authorization: Bearer $TOKEN" \
    https://www.lianel.se/api/v1/datasets/forecasting?limit=10

# 2. Restart Nginx
docker restart nginx-proxy

# 3. Monitor recovery
# - Check when Nginx becomes available
# - Check request handling
# - Check error rate
```

**Expected Behavior**:
- Requests should fail during restart
- Nginx should restart quickly (< 5 seconds)
- Requests should resume after restart

**Success Criteria**:
- [ ] Nginx restarts successfully
- [ ] Recovery time < 5 seconds
- [ ] No request loss after restart

---

### 6. Keycloak Failover

#### 6.1 Keycloak Restart

**Objective**: Test system behavior when Keycloak is restarted

**Test Procedure**:
```bash
# 1. Generate authentication requests
# (Multiple login attempts)

# 2. Restart Keycloak
docker restart keycloak

# 3. Monitor recovery
# - Check when Keycloak becomes available
# - Check authentication flow
# - Check token validation
```

**Expected Behavior**:
- Authentication requests should fail during restart
- Keycloak should restart within 1 minute
- Authentication should resume after restart

**Success Criteria**:
- [ ] Keycloak restarts successfully
- [ ] Recovery time < 1 minute
- [ ] Authentication works after restart

---

## Test Execution

### Pre-Test Checklist
- [ ] Backup system state
- [ ] Notify stakeholders
- [ ] Set up monitoring
- [ ] Document baseline metrics
- [ ] Prepare rollback plan

### During Test
- [ ] Monitor system metrics
- [ ] Record recovery times
- [ ] Document error messages
- [ ] Capture logs
- [ ] Note system behavior

### Post-Test
- [ ] Restore system state
- [ ] Analyze results
- [ ] Document findings
- [ ] Create recommendations
- [ ] Update runbooks

---

## Metrics to Monitor

### Recovery Time Objectives (RTO)
- **Service Restart**: < 30 seconds
- **Database Restart**: < 1 minute
- **Container Restart**: < 1 minute
- **Nginx Restart**: < 5 seconds
- **Keycloak Restart**: < 1 minute

### Recovery Metrics
- **Time to Recovery**: How long until service is fully operational
- **Error Rate During Failure**: Percentage of failed requests
- **Data Loss**: Any data lost during failover
- **Service Availability**: Percentage of time service is available

---

## Test Results Template

### For Each Test
```
Test: [Test Name]
Date: [Date]
Duration: [Duration]
Result: [Pass/Fail]

Recovery Time: [Time]
Error Rate: [Percentage]
Data Loss: [Yes/No]

Issues Found:
- [Issue 1]
- [Issue 2]

Recommendations:
- [Recommendation 1]
- [Recommendation 2]
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
- [ ] Start with non-critical services
- [ ] Have rollback plan ready
- [ ] Monitor closely
- [ ] Stop immediately if issues occur

---

## Tools

### Monitoring
- **Grafana**: Real-time metrics visualization
- **Prometheus**: Metrics collection
- **docker logs**: Container logs
- **docker stats**: Container resource usage

### Testing
- **wrk**: Load generation
- **curl**: Manual testing
- **docker**: Container management

---

## Success Criteria

- [ ] All failover scenarios tested
- [ ] Recovery times documented
- [ ] Recovery procedures validated
- [ ] RTO targets met
- [ ] No data loss during failover
- [ ] System recovers automatically

---

**Status**: Active  
**Last Review**: January 15, 2026
