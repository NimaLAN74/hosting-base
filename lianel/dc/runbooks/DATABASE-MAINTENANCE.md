# Database Maintenance Procedures
**Last Updated**: January 15, 2026  
**Owner**: Operations Team

---

## Overview

This runbook provides procedures for maintaining the PostgreSQL database used by the Lianel platform.

---

## Quick Reference

| Task | Command | Frequency |
|------|---------|-----------|
| Vacuum | `VACUUM ANALYZE;` | Weekly |
| Check connections | `SELECT count(*) FROM pg_stat_activity;` | Daily |
| Check table sizes | `SELECT pg_size_pretty(pg_total_relation_size('table_name'));` | Weekly |
| Backup | `pg_dump` | Daily |
| Check indexes | `SELECT * FROM pg_stat_user_indexes;` | Weekly |

---

## 1. Regular Maintenance Tasks

### 1.1 Vacuum and Analyze

#### Purpose
- Reclaim storage space
- Update query planner statistics
- Improve query performance

#### Procedure
```bash
# Connect to database
cd /root/lianel/dc
export POSTGRES_HOST=172.18.0.1
export POSTGRES_PORT=5432
export POSTGRES_USER=postgres
export POSTGRES_PASSWORD=$(grep POSTGRES_PASSWORD .env | cut -d'=' -f2)

# Vacuum and analyze all tables
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy <<EOF
VACUUM ANALYZE;
EOF

# Vacuum specific large tables
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy <<EOF
VACUUM ANALYZE fact_energy_annual;
VACUUM ANALYZE ml_dataset_forecasting_v1;
VACUUM ANALYZE ml_dataset_clustering_v1;
VACUUM ANALYZE ml_dataset_geo_enrichment_v1;
EOF
```

#### Frequency
- **Full VACUUM ANALYZE**: Weekly
- **ANALYZE only**: After large data loads
- **VACUUM only**: When disk space is low

---

### 1.2 Check Table Sizes

#### Procedure
```bash
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy <<EOF
SELECT 
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size,
    pg_size_pretty(pg_relation_size(schemaname||'.'||tablename)) AS table_size,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename) - pg_relation_size(schemaname||'.'||tablename)) AS indexes_size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC
LIMIT 20;
EOF
```

#### Action Items
- Monitor table growth
- Identify large tables for partitioning
- Plan storage capacity

---

### 1.3 Check Index Usage

#### Procedure
```bash
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy <<EOF
SELECT 
    schemaname,
    tablename,
    indexname,
    idx_scan as index_scans,
    idx_tup_read as tuples_read,
    idx_tup_fetch as tuples_fetched
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY idx_scan ASC;
EOF
```

#### Action Items
- Identify unused indexes (can be dropped)
- Monitor index effectiveness
- Add indexes for slow queries

---

### 1.4 Check Active Connections

#### Procedure
```bash
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy <<EOF
SELECT 
    count(*) as total_connections,
    count(*) FILTER (WHERE state = 'active') as active,
    count(*) FILTER (WHERE state = 'idle') as idle,
    count(*) FILTER (WHERE state = 'idle in transaction') as idle_in_transaction
FROM pg_stat_activity
WHERE datname = 'lianel_energy';
EOF
```

#### Action Items
- Monitor connection pool usage
- Identify long-running queries
- Check for connection leaks

---

## 2. Performance Monitoring

### 2.1 Slow Query Analysis

#### Procedure
```bash
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy <<EOF
SELECT 
    pid,
    now() - pg_stat_activity.query_start AS duration,
    state,
    query
FROM pg_stat_activity
WHERE state = 'active'
  AND now() - pg_stat_activity.query_start > interval '5 minutes'
ORDER BY duration DESC;
EOF
```

#### Action Items
- Identify slow queries
- Optimize or cancel if needed
- Add indexes if missing

---

### 2.2 Check Sequential Scans

#### Procedure
```bash
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy <<EOF
SELECT 
    schemaname,
    tablename,
    seq_scan,
    seq_tup_read,
    idx_scan,
    idx_tup_fetch,
    CASE WHEN seq_scan > 0 THEN (idx_scan::float / seq_scan::float) ELSE 0 END as index_ratio
FROM pg_stat_user_tables
WHERE schemaname = 'public'
  AND seq_scan > 100
ORDER BY seq_scan DESC
LIMIT 20;
EOF
```

#### Action Items
- Identify tables with high sequential scans
- Add indexes if needed
- Optimize queries

---

## 3. Backup Procedures

### 3.1 Manual Backup

#### Full Database Backup
```bash
cd /root/lianel/dc
export POSTGRES_HOST=172.18.0.1
export POSTGRES_PORT=5432
export POSTGRES_USER=postgres
export POSTGRES_PASSWORD=$(grep POSTGRES_PASSWORD .env | cut -d'=' -f2)

# Create backup directory
mkdir -p /root/backups/database
BACKUP_FILE="/root/backups/database/lianel_energy_$(date +%Y%m%d_%H%M%S).sql"

# Full backup
PGPASSWORD=${POSTGRES_PASSWORD} pg_dump -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy -F c -f ${BACKUP_FILE}

# Compress
gzip ${BACKUP_FILE}

# Verify backup
ls -lh ${BACKUP_FILE}.gz
```

#### Schema-Only Backup
```bash
PGPASSWORD=${POSTGRES_PASSWORD} pg_dump -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy --schema-only -f schema_backup.sql
```

#### Data-Only Backup
```bash
PGPASSWORD=${POSTGRES_PASSWORD} pg_dump -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy --data-only -F c -f data_backup.dump
```

---

### 3.2 Automated Backup Script

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

# Create backup
BACKUP_FILE="${BACKUP_DIR}/lianel_energy_$(date +%Y%m%d_%H%M%S).sql"
PGPASSWORD=${POSTGRES_PASSWORD} pg_dump -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy -F c -f ${BACKUP_FILE}

# Compress
gzip ${BACKUP_FILE}

echo "Backup created: ${BACKUP_FILE}.gz"
```

---

### 3.3 Backup Verification

#### Procedure
```bash
# List backups
ls -lh /root/backups/database/

# Verify backup file
gunzip -t backup_file.sql.gz

# Test restore (to different database)
PGPASSWORD=${POSTGRES_PASSWORD} createdb -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} test_restore
gunzip -c backup_file.sql.gz | PGPASSWORD=${POSTGRES_PASSWORD} pg_restore -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d test_restore
```

---

## 4. Recovery Procedures

### 4.1 Restore from Backup

#### Full Restore
```bash
# WARNING: This will overwrite existing data!

cd /root/lianel/dc
export POSTGRES_HOST=172.18.0.1
export POSTGRES_PORT=5432
export POSTGRES_USER=postgres
export POSTGRES_PASSWORD=$(grep POSTGRES_PASSWORD .env | cut -d'=' -f2)

BACKUP_FILE="/root/backups/database/lianel_energy_YYYYMMDD_HHMMSS.sql.gz"

# Stop services that use database
docker compose -f docker-compose.airflow.yaml stop
docker compose -f docker-compose.yaml stop

# Drop and recreate database (CAREFUL!)
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d postgres <<EOF
DROP DATABASE IF EXISTS lianel_energy;
CREATE DATABASE lianel_energy;
EOF

# Restore backup
gunzip -c ${BACKUP_FILE} | PGPASSWORD=${POSTGRES_PASSWORD} pg_restore -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy

# Restart services
docker compose -f docker-compose.airflow.yaml start
docker compose -f docker-compose.yaml start
```

#### Partial Restore (Single Table)
```bash
# Restore specific table
gunzip -c ${BACKUP_FILE} | PGPASSWORD=${POSTGRES_PASSWORD} pg_restore -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy -t table_name
```

---

## 5. Index Maintenance

### 5.1 Rebuild Indexes

#### Procedure
```bash
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy <<EOF
-- Rebuild specific index
REINDEX INDEX index_name;

-- Rebuild all indexes on table
REINDEX TABLE table_name;

-- Rebuild all indexes in database (takes time!)
REINDEX DATABASE lianel_energy;
EOF
```

#### When to Rebuild
- After large data loads
- If indexes are corrupted
- Performance degradation
- Monthly maintenance

---

### 5.2 Add Missing Indexes

#### Procedure
```bash
# Run performance analysis script
cd /root/lianel/dc
bash scripts/analyze-db-performance.sh

# Review output for missing indexes
# Add indexes as needed via migrations
```

---

## 6. Connection Management

### 6.1 Check Connection Limits

#### Procedure
```bash
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy <<EOF
SHOW max_connections;
SELECT count(*) as current_connections FROM pg_stat_activity;
EOF
```

### 6.2 Kill Long-Running Queries

#### Procedure
```bash
# Find long-running queries
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy <<EOF
SELECT pid, now() - query_start AS duration, query
FROM pg_stat_activity
WHERE state = 'active'
  AND now() - query_start > interval '10 minutes';
EOF

# Kill specific query (CAREFUL!)
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy <<EOF
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE pid = <process_id>;
EOF
```

---

## 7. Maintenance Schedule

### Daily
- Check active connections
- Monitor table sizes
- Review slow queries

### Weekly
- VACUUM ANALYZE
- Check index usage
- Review backup status

### Monthly
- Full database backup verification
- Rebuild indexes if needed
- Review and optimize queries
- Check for unused indexes

### Quarterly
- Review maintenance procedures
- Update documentation
- Performance tuning
- Capacity planning

---

## 8. Troubleshooting

### Database Locked
```bash
# Check for locks
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy <<EOF
SELECT * FROM pg_locks WHERE NOT granted;
EOF

# Kill blocking queries if needed
```

### High Connection Count
```bash
# Check connection sources
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy <<EOF
SELECT application_name, count(*) 
FROM pg_stat_activity 
GROUP BY application_name;
EOF
```

### Disk Space Issues
```bash
# Check database size
PGPASSWORD=${POSTGRES_PASSWORD} psql -h ${POSTGRES_HOST} -p ${POSTGRES_PORT} -U ${POSTGRES_USER} -d lianel_energy <<EOF
SELECT pg_size_pretty(pg_database_size('lianel_energy'));
EOF

# Vacuum to reclaim space
VACUUM FULL;  # Use with caution - locks tables
```

---

## 9. Quick Commands Reference

```bash
# Connect to database
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy

# Vacuum and analyze
VACUUM ANALYZE;

# Check table sizes
SELECT pg_size_pretty(pg_total_relation_size('table_name'));

# Backup
pg_dump -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy -F c -f backup.dump

# Restore
pg_restore -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy backup.dump

# Check connections
SELECT count(*) FROM pg_stat_activity;

# Kill query
SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE pid = <pid>;
```

---

**Status**: Active  
**Review Frequency**: Quarterly  
**Last Review**: January 15, 2026
