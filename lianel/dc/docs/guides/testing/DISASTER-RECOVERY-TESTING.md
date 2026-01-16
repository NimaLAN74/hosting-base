# Disaster Recovery Testing Guide
**Last Updated**: January 15, 2026

---

## Overview

Disaster recovery testing validates that the system can recover from catastrophic failures and restore operations within defined recovery time objectives (RTO) and recovery point objectives (RPO).

---

## Objectives

1. Test backup restoration
2. Test full system recovery
3. Test partial recovery scenarios
4. Validate RTO and RPO targets
5. Document recovery procedures
6. Identify gaps in recovery capabilities

---

## Recovery Objectives

### Recovery Time Objectives (RTO)
- **Full System Recovery**: < 4 hours
- **Critical Services**: < 2 hours
- **Database Recovery**: < 1 hour
- **Configuration Recovery**: < 30 minutes

### Recovery Point Objectives (RPO)
- **Database**: < 1 hour (hourly backups)
- **Configuration**: < 24 hours (daily backups)
- **Application Code**: < 1 week (version control)

---

## Test Scenarios

### 1. Full System Recovery

#### 1.1 Complete System Failure

**Scenario**: Complete system failure requiring full restoration

**Test Procedure**:
```bash
# 1. Backup current state
./scripts/backup-database.sh
tar -czf backup-$(date +%Y%m%d).tar.gz .env docker-compose*.yaml

# 2. Simulate complete failure
# Stop all services
docker-compose -f docker-compose.infra.yaml down
docker-compose -f docker-compose.yaml down
docker-compose -f docker-compose.airflow.yaml down
docker-compose -f docker-compose.monitoring.yaml down
docker-compose -f docker-compose.oauth2-proxy.yaml down

# Remove volumes (simulate data loss)
docker volume prune -f

# 3. Restore from backup
# Restore database
./scripts/restore-database.sh backup-YYYYMMDD.sql

# Restore configuration
tar -xzf backup-YYYYMMDD.tar.gz -C /root/hosting-base/lianel/dc

# 4. Rebuild and restart
docker-compose -f docker-compose.infra.yaml up -d
sleep 30
docker-compose -f docker-compose.yaml up -d
docker-compose -f docker-compose.airflow.yaml up -d
docker-compose -f docker-compose.monitoring.yaml up -d
docker-compose -f docker-compose.oauth2-proxy.yaml up -d

# 5. Verify recovery
curl https://www.lianel.se/api/v1/health
curl https://auth.lianel.se
curl https://airflow.lianel.se
```

**Expected Behavior**:
- System should be restored from backups
- All services should start successfully
- Data should be consistent
- System should be fully operational

**Success Criteria**:
- [ ] All services restored
- [ ] Data integrity verified
- [ ] System fully operational
- [ ] RTO target met (< 4 hours)

---

#### 1.2 Server Failure

**Scenario**: Complete server failure requiring migration to new server

**Test Procedure**:
```bash
# 1. On new server, install prerequisites
sudo apt update && sudo apt upgrade -y
sudo apt install -y docker.io docker-compose postgresql

# 2. Clone repository
git clone <repository-url> hosting-base
cd hosting-base/lianel/dc

# 3. Restore backups
# Transfer backups from old server
scp user@old-server:/root/backups/* /root/backups/

# Restore database
./scripts/restore-database.sh /root/backups/backup-YYYYMMDD.sql

# Restore configuration
tar -xzf /root/backups/backup-YYYYMMDD.tar.gz -C /root/hosting-base/lianel/dc

# 4. Configure environment
cp .env.example .env
# Edit .env with production values

# 5. Deploy services
docker network create lianel-network
docker-compose -f docker-compose.infra.yaml up -d
sleep 30
docker-compose -f docker-compose.yaml up -d
docker-compose -f docker-compose.airflow.yaml up -d
docker-compose -f docker-compose.monitoring.yaml up -d
docker-compose -f docker-compose.oauth2-proxy.yaml up -d

# 6. Verify recovery
curl https://www.lianel.se/api/v1/health
```

**Expected Behavior**:
- New server should be configured
- Services should be deployed
- Data should be restored
- System should be operational

**Success Criteria**:
- [ ] New server configured
- [ ] Services deployed
- [ ] Data restored
- [ ] System operational
- [ ] RTO target met (< 4 hours)

---

### 2. Database Recovery

#### 2.1 Database Corruption

**Scenario**: Database corruption requiring restoration from backup

**Test Procedure**:
```bash
# 1. Backup current database
./scripts/backup-database.sh

# 2. Simulate corruption
# (In test environment only!)
sudo -u postgres psql -d airflow -c "DROP TABLE dag_run;"

# 3. Restore from backup
./scripts/restore-database.sh backup-YYYYMMDD.sql

# 4. Verify restoration
psql -h localhost -U airflow -d airflow -c "SELECT count(*) FROM dag_run;"

# 5. Restart services
docker-compose -f docker-compose.airflow.yaml restart
```

**Expected Behavior**:
- Database should be restored from backup
- Data should be consistent
- Services should resume operation

**Success Criteria**:
- [ ] Database restored
- [ ] Data integrity verified
- [ ] Services operational
- [ ] RTO target met (< 1 hour)

---

#### 2.2 Database Point-in-Time Recovery

**Scenario**: Restore database to specific point in time

**Test Procedure**:
```bash
# 1. Identify recovery point
# Check backup timestamps
ls -lh /root/backups/

# 2. Restore to specific backup
./scripts/restore-database.sh backup-YYYYMMDD-HHMMSS.sql

# 3. Verify data at recovery point
psql -h localhost -U airflow -d airflow -c "SELECT max(execution_date) FROM dag_run;"

# 4. Restart services
docker-compose -f docker-compose.airflow.yaml restart
```

**Expected Behavior**:
- Database should be restored to specified point
- Data should match recovery point
- Services should resume operation

**Success Criteria**:
- [ ] Database restored to recovery point
- [ ] Data matches recovery point
- [ ] RPO target met (< 1 hour)

---

### 3. Configuration Recovery

#### 3.1 Configuration Loss

**Scenario**: Configuration files lost or corrupted

**Test Procedure**:
```bash
# 1. Backup current configuration
tar -czf config-backup-$(date +%Y%m%d).tar.gz .env docker-compose*.yaml

# 2. Simulate configuration loss
rm .env docker-compose*.yaml

# 3. Restore from backup
tar -xzf config-backup-YYYYMMDD.tar.gz

# 4. Verify restoration
cat .env | grep -v "PASSWORD"  # Review (don't show passwords)

# 5. Restart services
docker-compose -f docker-compose.infra.yaml up -d
docker-compose -f docker-compose.yaml up -d
```

**Expected Behavior**:
- Configuration should be restored
- Services should start with restored configuration
- System should be operational

**Success Criteria**:
- [ ] Configuration restored
- [ ] Services operational
- [ ] RTO target met (< 30 minutes)

---

### 4. Partial Recovery

#### 4.1 Service-Specific Recovery

**Scenario**: Single service failure requiring recovery

**Test Procedure**:
```bash
# 1. Stop and remove service
docker-compose -f docker-compose.yaml stop lianel-energy-service
docker-compose -f docker-compose.yaml rm -f lianel-energy-service

# 2. Restore service data (if applicable)
# (Service-specific restoration)

# 3. Restart service
docker-compose -f docker-compose.yaml up -d lianel-energy-service

# 4. Verify recovery
curl https://www.lianel.se/api/v1/health
```

**Expected Behavior**:
- Service should be restored
- Service should resume operation
- Other services should be unaffected

**Success Criteria**:
- [ ] Service restored
- [ ] Service operational
- [ ] Other services unaffected

---

## Test Execution

### Pre-Test Checklist
- [ ] Backup current system state
- [ ] Document current configuration
- [ ] Notify stakeholders
- [ ] Set up monitoring
- [ ] Prepare rollback plan
- [ ] Schedule test window

### During Test
- [ ] Document recovery steps
- [ ] Record recovery times
- [ ] Monitor system behavior
- [ ] Capture logs
- [ ] Note issues and gaps

### Post-Test
- [ ] Restore system to normal state
- [ ] Analyze results
- [ ] Document findings
- [ ] Create recommendations
- [ ] Update recovery procedures
- [ ] Update RTO/RPO targets if needed

---

## Metrics to Monitor

### Recovery Time Metrics
- **Time to Detect**: How long until failure is detected
- **Time to Respond**: How long until recovery starts
- **Time to Recover**: How long until system is operational
- **Total Recovery Time**: End-to-end recovery time

### Data Loss Metrics
- **Data Loss Window**: Time between last backup and failure
- **Data Loss Amount**: Amount of data lost
- **Data Integrity**: Verification of restored data

### Service Availability
- **Downtime Duration**: Total time system was unavailable
- **Service Availability**: Percentage of time services were available
- **Impact Assessment**: Impact on users and operations

---

## Test Results Template

### For Each Test
```
Test: [Test Name]
Date: [Date]
Duration: [Duration]
Result: [Pass/Fail]

RTO: [Actual Time] / [Target Time]
RPO: [Actual Time] / [Target Time]
Data Loss: [Amount]

Issues Found:
- [Issue 1]
- [Issue 2]

Gaps Identified:
- [Gap 1]
- [Gap 2]

Recommendations:
- [Recommendation 1]
- [Recommendation 2]

Lessons Learned:
- [Lesson 1]
- [Lesson 2]
```

---

## Backup Verification

### Regular Backup Testing
- [ ] Test backup restoration monthly
- [ ] Verify backup integrity
- [ ] Test backup procedures
- [ ] Document backup test results

### Backup Validation
```bash
# Verify backup file integrity
tar -tzf backup-YYYYMMDD.tar.gz

# Verify database backup
pg_restore --list backup-YYYYMMDD.sql | head -20

# Test backup restoration (in test environment)
./scripts/restore-database.sh backup-YYYYMMDD.sql
```

---

## Recovery Procedures

### Full System Recovery
1. Restore database from backup
2. Restore configuration files
3. Rebuild Docker images
4. Deploy services
5. Verify system operation

### Partial Recovery
1. Identify affected components
2. Restore affected components
3. Verify component operation
4. Verify system integration

---

## Safety Considerations

### Test Environment
- [ ] Use dedicated test environment
- [ ] Isolate from production
- [ ] Have rollback plan ready
- [ ] Monitor closely
- [ ] Stop test if critical issues occur

### Production Testing
- [ ] Schedule during maintenance window
- [ ] Have rollback plan ready
- [ ] Monitor closely
- [ ] Stop immediately if issues occur
- [ ] Document all steps

---

## Tools

### Backup Tools
- **pg_dump**: PostgreSQL backup
- **tar**: Configuration backup
- **rsync**: File synchronization

### Recovery Tools
- **pg_restore**: PostgreSQL restoration
- **docker-compose**: Service deployment
- **git**: Code restoration

### Monitoring
- **Grafana**: System monitoring
- **Prometheus**: Metrics collection
- **docker logs**: Service logs

---

## Success Criteria

- [ ] All recovery scenarios tested
- [ ] RTO targets met
- [ ] RPO targets met
- [ ] Recovery procedures validated
- [ ] No data loss beyond RPO
- [ ] System fully operational after recovery

---

## Continuous Improvement

### Regular Review
- [ ] Review RTO/RPO targets quarterly
- [ ] Update recovery procedures
- [ ] Test new recovery scenarios
- [ ] Update documentation

### Lessons Learned
- [ ] Document lessons learned
- [ ] Update procedures based on findings
- [ ] Share knowledge with team
- [ ] Improve recovery capabilities

---

**Status**: Active  
**Last Review**: January 15, 2026
