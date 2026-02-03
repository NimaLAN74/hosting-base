# Backup and Recovery Procedures
**Last Updated**: January 15, 2026  
**Owner**: Operations Team

---

## Overview

This document defines procedures for backing up and recovering the Lianel platform.

---

## 1. Backup Strategy

### 1.1 Backup Types

#### Full Backup
- Complete database dump
- All configuration files
- All data files
- Frequency: Daily

#### Incremental Backup
- Changes since last backup
- Configuration changes
- Frequency: As needed

#### Point-in-Time Recovery
- Database transaction logs
- WAL archiving
- Frequency: Continuous (if enabled)

---

### 1.2 Backup Schedule

| Component | Frequency | Retention | Location |
|-----------|-----------|-----------|----------|
| Database | Daily | 7 days | `/root/backups/database` |
| Configuration | Weekly | 30 days | `/root/backups/config` |
| Docker Volumes | Weekly | 14 days | `/root/backups/volumes` |
| Logs | Daily | 7 days | `/root/backups/logs` |

---

## 2. Database Backup

### 2.1 Automated Backup

#### Backup Script
Create `/root/lianel/dc/scripts/backup-database.sh`:
```bash
#!/bin/bash
# Database Backup Script

set -euo pipefail

cd /root/lianel/dc
export POSTGRES_HOST=172.18.0.1
export POSTGRES_PORT=5432
export POSTGRES_USER=postgres
export POSTGRES_PASSWORD=$(grep POSTGRES_PASSWORD .env | cut -d'=' -f2)

BACKUP_DIR="/root/backups/database"
mkdir -p ${BACKUP_DIR}

# Keep only last 7 days of backups
find ${BACKUP_DIR} -name "*.sql.gz" -mtime +7 -delete

# Create backup (day-first)
BACKUP_FILE="${BACKUP_DIR}/lianel_energy_$(date +%d-%m-%Y_%H%M%S).sql"
PGPASSWORD=${POSTGRES_PASSWORD} pg_dump -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy -F c -f ${BACKUP_FILE}

# Compress
gzip ${BACKUP_FILE}

# Verify backup
if [ -f "${BACKUP_FILE}.gz" ]; then
    echo "Backup created: ${BACKUP_FILE}.gz"
    ls -lh ${BACKUP_FILE}.gz
else
    echo "ERROR: Backup failed!"
    exit 1
fi
```

#### Schedule with Cron
```bash
# Add to crontab
crontab -e

# Daily backup at 2 AM
0 2 * * * /root/lianel/dc/scripts/backup-database.sh >> /var/log/backup-database.log 2>&1
```

---

### 2.2 Manual Backup

#### Full Database Backup
```bash
cd /root/lianel/dc
export POSTGRES_HOST=172.18.0.1
export POSTGRES_PORT=5432
export POSTGRES_USER=postgres
export POSTGRES_PASSWORD=$(grep POSTGRES_PASSWORD .env | cut -d'=' -f2)

BACKUP_DIR="/root/backups/database"
mkdir -p ${BACKUP_DIR}
BACKUP_FILE="${BACKUP_DIR}/lianel_energy_$(date +%d-%m-%Y_%H%M%S).sql"

# Full backup
PGPASSWORD=${POSTGRES_PASSWORD} pg_dump -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy -F c -f ${BACKUP_FILE}

# Compress
gzip ${BACKUP_FILE}

# Verify
ls -lh ${BACKUP_FILE}.gz
```

#### Schema-Only Backup
```bash
SCHEMA_FILE="${BACKUP_DIR}/schema_$(date +%d-%m-%Y_%H%M%S).sql"
PGPASSWORD=${POSTGRES_PASSWORD} pg_dump -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy --schema-only -f ${SCHEMA_FILE}
```

#### Data-Only Backup
```bash
DATA_FILE="${BACKUP_DIR}/data_$(date +%d-%m-%Y_%H%M%S).dump"
PGPASSWORD=${POSTGRES_PASSWORD} pg_dump -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy --data-only -F c -f ${DATA_FILE}
```

---

### 2.3 Backup Verification

#### Verify Backup File
```bash
# Check file exists and size
ls -lh /root/backups/database/*.sql.gz

# Test compression
gunzip -t backup_file.sql.gz

# Check backup contents
gunzip -c backup_file.sql.gz | head -100
```

#### Test Restore (to test database)
```bash
# Create test database
PGPASSWORD=${POSTGRES_PASSWORD} createdb -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} test_restore

# Restore backup
gunzip -c backup_file.sql.gz | PGPASSWORD=${POSTGRES_PASSWORD} pg_restore -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d test_restore

# Verify restore
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d test_restore -c "SELECT COUNT(*) FROM fact_energy_annual;"

# Drop test database
PGPASSWORD=${POSTGRES_PASSWORD} dropdb -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} test_restore
```

---

## 3. Configuration Backup

### 3.1 Backup Configuration Files

#### Procedure
```bash
BACKUP_DIR="/root/backups/config"
mkdir -p ${BACKUP_DIR}
CONFIG_BACKUP="${BACKUP_DIR}/config_$(date +%d-%m-%Y_%H%M%S).tar.gz"

# Backup configuration
tar -czf ${CONFIG_BACKUP} \
    /root/lianel/dc/.env \
    /root/lianel/dc/docker-compose*.yaml \
    /root/lianel/dc/nginx/config/ \
    /root/lianel/dc/monitoring/prometheus/ \
    /root/lianel/dc/monitoring/grafana/provisioning/ \
    /root/lianel/dc/dags/

# Verify
ls -lh ${CONFIG_BACKUP}
```

#### Schedule
```bash
# Weekly backup
0 3 * * 0 /root/lianel/dc/scripts/backup-config.sh
```

---

### 3.2 Backup Environment Variables

#### Procedure
```bash
# Backup .env file (careful with secrets!)
BACKUP_DIR="/root/backups/config"
mkdir -p ${BACKUP_DIR}
cp /root/lianel/dc/.env ${BACKUP_DIR}/.env.$(date +%d-%m-%Y_%H%M%S)

# Keep only last 30 days
find ${BACKUP_DIR} -name ".env.*" -mtime +30 -delete
```

---

## 4. Docker Volumes Backup

### 4.1 Backup Volumes

#### Procedure
```bash
BACKUP_DIR="/root/backups/volumes"
mkdir -p ${BACKUP_DIR}
VOLUME_BACKUP="${BACKUP_DIR}/volumes_$(date +%d-%m-%Y_%H%M%S).tar.gz"

# List volumes
docker volume ls | grep lianel

# Backup volumes (example)
docker run --rm \
    -v lianel_grafana_data:/data \
    -v ${BACKUP_DIR}:/backup \
    alpine tar -czf /backup/grafana_data_$(date +%d-%m-%Y_%H%M%S).tar.gz -C /data .
```

---

## 5. Recovery Procedures

### 5.1 Database Recovery

#### Full Database Restore

**WARNING**: This will overwrite existing data!

```bash
cd /root/lianel/dc
export POSTGRES_HOST=172.18.0.1
export POSTGRES_PORT=5432
export POSTGRES_USER=postgres
export POSTGRES_PASSWORD=$(grep POSTGRES_PASSWORD .env | cut -d'=' -f2)

BACKUP_FILE="/root/backups/database/lianel_energy_YYYYMMDD_HHMMSS.sql.gz"

# 1. Stop services that use database
docker compose -f docker-compose.airflow.yaml stop
docker compose -f docker-compose.yaml stop

# 2. Drop and recreate database (CAREFUL!)
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d postgres <<EOF
DROP DATABASE IF EXISTS lianel_energy;
CREATE DATABASE lianel_energy;
EOF

# 3. Restore backup
gunzip -c ${BACKUP_FILE} | PGPASSWORD=${POSTGRES_PASSWORD} pg_restore -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy

# 4. Verify restore
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy -c "SELECT COUNT(*) FROM fact_energy_annual;"

# 5. Restart services
docker compose -f docker-compose.airflow.yaml start
docker compose -f docker-compose.yaml start
```

---

#### Partial Restore (Single Table)

```bash
# Restore specific table
gunzip -c ${BACKUP_FILE} | PGPASSWORD=${POSTGRES_PASSWORD} pg_restore -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy -t table_name
```

---

#### Point-in-Time Recovery

If WAL archiving is enabled:

```bash
# 1. Stop PostgreSQL
systemctl stop postgresql

# 2. Restore base backup
# (Follow PostgreSQL PITR documentation)

# 3. Replay WAL logs to target time
# (Follow PostgreSQL PITR documentation)

# 4. Start PostgreSQL
systemctl start postgresql
```

---

### 5.2 Configuration Recovery

#### Procedure
```bash
CONFIG_BACKUP="/root/backups/config/config_YYYYMMDD_HHMMSS.tar.gz"

# Extract configuration
cd /root/lianel/dc
tar -xzf ${CONFIG_BACKUP}

# Verify configuration
docker compose -f docker-compose.yaml config

# Restart services if needed
docker compose -f docker-compose.yaml restart
```

---

### 5.3 Volume Recovery

#### Procedure
```bash
VOLUME_BACKUP="/root/backups/volumes/grafana_data_YYYYMMDD_HHMMSS.tar.gz"

# Stop service
docker compose stop grafana

# Restore volume
docker run --rm \
    -v lianel_grafana_data:/data \
    -v /root/backups/volumes:/backup \
    alpine sh -c "cd /data && rm -rf * && tar -xzf /backup/grafana_data_YYYYMMDD_HHMMSS.tar.gz"

# Start service
docker compose start grafana
```

---

## 6. Disaster Recovery Plan

### 6.1 Disaster Scenarios

#### Complete System Failure
- Host failure
- Data center outage
- Complete data loss

#### Partial Failure
- Database corruption
- Configuration loss
- Service failures

---

### 6.2 Recovery Steps

#### Step 1: Assess Damage
- Identify affected systems
- Determine data loss
- Check backup availability
- Assess recovery time

#### Step 2: Restore Infrastructure
- Provision new host if needed
- Install Docker and dependencies
- Restore network configuration
- Verify connectivity

#### Step 3: Restore Data
- Restore database from backup
- Restore configuration files
- Restore volumes if needed
- Verify data integrity

#### Step 4: Restore Services
- Start infrastructure services
- Start application services
- Start monitoring services
- Verify service health

#### Step 5: Verification
- Test all functionality
- Verify data integrity
- Check monitoring
- Document recovery

---

### 6.3 Recovery Time Objectives (RTO)

| Component | RTO | Backup Frequency |
|-----------|-----|------------------|
| Database | 2 hours | Daily |
| Configuration | 1 hour | Weekly |
| Application | 1 hour | N/A |
| Full System | 4 hours | Daily |

---

### 6.4 Recovery Point Objectives (RPO)

| Component | RPO | Backup Frequency |
|-----------|-----|------------------|
| Database | 24 hours | Daily |
| Configuration | 7 days | Weekly |
| Logs | 7 days | Daily |

---

## 7. Backup Testing

### 7.1 Regular Testing

#### Monthly Tests
- Verify backup files exist
- Test backup restoration
- Verify data integrity
- Document test results

#### Quarterly Tests
- Full disaster recovery drill
- Test recovery procedures
- Update documentation
- Review and improve

---

### 7.2 Test Procedure

```bash
# 1. Create test database
PGPASSWORD=${POSTGRES_PASSWORD} createdb -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} test_restore

# 2. Restore latest backup
LATEST_BACKUP=$(ls -t /root/backups/database/*.sql.gz | head -1)
gunzip -c ${LATEST_BACKUP} | PGPASSWORD=${POSTGRES_PASSWORD} pg_restore -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d test_restore

# 3. Verify data
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d test_restore <<EOF
SELECT COUNT(*) FROM fact_energy_annual;
SELECT COUNT(*) FROM fact_geo_region_features;
SELECT MAX(ingestion_date) FROM meta_ingestion_log;
EOF

# 4. Clean up
PGPASSWORD=${POSTGRES_PASSWORD} dropdb -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} test_restore
```

---

## 8. Backup Monitoring

### 8.1 Backup Status Checks

#### Daily Checks
```bash
# Check latest backup
ls -lh /root/backups/database/*.sql.gz | tail -1

# Check backup age
find /root/backups/database -name "*.sql.gz" -mtime +1

# Check backup size
du -sh /root/backups/database/
```

#### Automated Monitoring
- Add backup status to monitoring
- Alert if backup fails
- Alert if backup is old
- Alert if backup is too small

---

## 9. Quick Commands Reference

```bash
# Create database backup
PGPASSWORD=${POSTGRES_PASSWORD} pg_dump -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy -F c -f backup.dump
gzip backup.dump

# Restore database backup
gunzip -c backup.dump.gz | PGPASSWORD=${POSTGRES_PASSWORD} pg_restore -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy

# Backup configuration
tar -czf config_backup.tar.gz /root/lianel/dc/.env /root/lianel/dc/docker-compose*.yaml

# List backups
ls -lh /root/backups/database/
ls -lh /root/backups/config/

# Verify backup
gunzip -t backup_file.sql.gz

# Test restore
PGPASSWORD=${POSTGRES_PASSWORD} createdb test_restore
gunzip -c backup_file.sql.gz | PGPASSWORD=${POSTGRES_PASSWORD} pg_restore -d test_restore
```

---

## 10. Backup Retention

### 10.1 Retention Policy

| Backup Type | Retention | Location |
|-------------|-----------|----------|
| Daily backups | 7 days | Local |
| Weekly backups | 4 weeks | Local |
| Monthly backups | 3 months | Local + Remote |
| Configuration | 30 days | Local |

### 10.2 Cleanup

```bash
# Remove old database backups (>7 days)
find /root/backups/database -name "*.sql.gz" -mtime +7 -delete

# Remove old config backups (>30 days)
find /root/backups/config -name "*.tar.gz" -mtime +30 -delete
```

---

**Status**: Active  
**Review Frequency**: Quarterly  
**Last Review**: January 15, 2026
