# Rollback Procedures
**Last Updated**: January 15, 2026

---

## Overview

This document outlines procedures for rolling back deployments when issues are detected in production.

---

## Rollback Decision Criteria

### Automatic Rollback Triggers
- Error rate > 5% for 5 minutes
- Service unavailability > 2 minutes
- Database connection failures > 10%
- Critical security vulnerability detected

### Manual Rollback Triggers
- Data corruption detected
- Performance degradation > 50%
- User-reported critical issues
- Security incident

---

## Pre-Rollback Checklist

- [ ] Identify the issue and root cause
- [ ] Determine rollback scope (full or partial)
- [ ] Verify backup availability
- [ ] Notify stakeholders
- [ ] Document the issue

---

## Rollback Procedures

### Full System Rollback

#### Step 1: Stop Current Services
```bash
cd /root/hosting-base/lianel/dc

# Stop all services
docker-compose -f docker-compose.infra.yaml down
docker-compose -f docker-compose.yaml down
docker-compose -f docker-compose.airflow.yaml down
docker-compose -f docker-compose.monitoring.yaml down
docker-compose -f docker-compose.oauth2-proxy.yaml down
```

#### Step 2: Restore Previous Version
```bash
# Checkout previous version
cd /root/hosting-base
git log --oneline -10  # Review commits
git checkout <previous-commit-hash>

# Or restore from backup
cd /root
tar -xzf backup-YYYYMMDD.tar.gz -C hosting-base/lianel/dc
```

#### Step 3: Restore Database (if needed)
```bash
# Restore PostgreSQL databases
./scripts/restore-database.sh backup-YYYYMMDD.sql
```

#### Step 4: Restore Configuration
```bash
# Restore .env file
cp backup/.env /root/hosting-base/lianel/dc/.env

# Verify configuration
cat .env | grep -v "PASSWORD"  # Review (don't show passwords)
```

#### Step 5: Rebuild and Deploy
```bash
cd /root/hosting-base/lianel/dc

# Rebuild images if needed
docker-compose -f docker-compose.yaml build

# Deploy services
docker-compose -f docker-compose.infra.yaml up -d
sleep 30
docker-compose -f docker-compose.yaml up -d
docker-compose -f docker-compose.airflow.yaml up -d
docker-compose -f docker-compose.monitoring.yaml up -d
docker-compose -f docker-compose.oauth2-proxy.yaml up -d
```

#### Step 6: Verify Rollback
```bash
# Check service status
docker ps

# Verify endpoints
curl https://www.lianel.se/api/v1/health
curl https://auth.lianel.se
curl https://airflow.lianel.se

# Check logs for errors
docker logs <service-name> --tail 50
```

---

### Partial Rollback (Service-Specific)

#### Rollback Frontend
```bash
# Stop frontend
docker-compose -f docker-compose.yaml stop frontend

# Checkout previous version
cd /root/hosting-base
git checkout <previous-commit> -- lianel/dc/frontend/

# Rebuild and restart
cd lianel/dc
docker-compose -f docker-compose.yaml build frontend
docker-compose -f docker-compose.yaml up -d frontend
```

#### Rollback Backend Service
```bash
# Stop service
docker-compose -f docker-compose.yaml stop lianel-energy-service

# Checkout previous version
cd /root/hosting-base
git checkout <previous-commit> -- lianel/dc/energy-service/

# Rebuild and restart
cd lianel/dc
docker-compose -f docker-compose.yaml build lianel-energy-service
docker-compose -f docker-compose.yaml up -d lianel-energy-service
```

#### Rollback Airflow
```bash
# Stop Airflow
docker-compose -f docker-compose.airflow.yaml down

# Checkout previous version
cd /root/hosting-base
git checkout <previous-commit> -- lianel/dc/dags/

# Restart Airflow
cd lianel/dc
docker-compose -f docker-compose.airflow.yaml up -d
```

---

### Database Rollback

#### Full Database Restore
```bash
# Stop services using database
docker-compose -f docker-compose.yaml stop
docker-compose -f docker-compose.airflow.yaml stop

# Restore database
./scripts/restore-database.sh backup-YYYYMMDD.sql

# Restart services
docker-compose -f docker-compose.yaml up -d
docker-compose -f docker-compose.airflow.yaml up -d
```

#### Partial Database Restore (Table-Specific)
```bash
# Connect to database
psql -h localhost -U airflow -d airflow

# Restore specific table
\c airflow
DROP TABLE IF EXISTS table_name;
\i backup-table_name.sql
```

---

## Blue-Green Deployment Rollback

If using blue-green deployment:

### Rollback to Previous Environment
```bash
# Switch traffic to previous environment
# Update Nginx configuration to point to previous upstream

# Reload Nginx
docker exec nginx-proxy nginx -s reload

# Verify traffic is routed correctly
curl -I https://www.lianel.se
```

### Cleanup Failed Environment
```bash
# Stop failed environment
docker-compose -f docker-compose.blue.yaml down  # or green.yaml

# Remove containers and volumes if needed
docker-compose -f docker-compose.blue.yaml down -v
```

---

## Canary Release Rollback

If using canary releases:

### Rollback Canary
```bash
# Reduce canary traffic to 0%
# Update load balancer configuration

# Stop canary deployment
docker-compose -f docker-compose.canary.yaml down

# Verify all traffic is on stable version
```

---

## Post-Rollback Tasks

### Immediate
- [ ] Verify all services are running
- [ ] Test critical functionality
- [ ] Monitor error rates
- [ ] Check system health

### Short-term
- [ ] Document the issue and root cause
- [ ] Create incident report
- [ ] Review rollback procedure effectiveness
- [ ] Update procedures if needed

### Long-term
- [ ] Fix the issue in development
- [ ] Test fix thoroughly
- [ ] Plan re-deployment
- [ ] Update monitoring/alerts

---

## Rollback Scripts

### Quick Rollback Script
```bash
#!/bin/bash
# quick-rollback.sh

BACKUP_DATE=$1
if [ -z "$BACKUP_DATE" ]; then
    echo "Usage: $0 YYYYMMDD"
    exit 1
fi

cd /root/hosting-base/lianel/dc

# Stop all services
echo "Stopping services..."
docker-compose -f docker-compose.infra.yaml down
docker-compose -f docker-compose.yaml down
docker-compose -f docker-compose.airflow.yaml down
docker-compose -f docker-compose.monitoring.yaml down
docker-compose -f docker-compose.oauth2-proxy.yaml down

# Restore from backup
echo "Restoring from backup..."
tar -xzf /root/backups/backup-${BACKUP_DATE}.tar.gz -C /root/hosting-base/lianel/dc

# Restore database
echo "Restoring database..."
./scripts/restore-database.sh /root/backups/backup-${BACKUP_DATE}.sql

# Restart services
echo "Restarting services..."
docker-compose -f docker-compose.infra.yaml up -d
sleep 30
docker-compose -f docker-compose.yaml up -d
docker-compose -f docker-compose.airflow.yaml up -d
docker-compose -f docker-compose.monitoring.yaml up -d
docker-compose -f docker-compose.oauth2-proxy.yaml up -d

echo "Rollback complete. Verify services:"
docker ps
```

---

## Testing Rollback Procedures

### Regular Testing
- [ ] Test rollback procedures quarterly
- [ ] Document test results
- [ ] Update procedures based on findings
- [ ] Train team on rollback procedures

### Test Scenarios
1. **Service Failure**: Simulate service crash and rollback
2. **Database Corruption**: Simulate database issue and restore
3. **Configuration Error**: Simulate config issue and rollback
4. **Performance Degradation**: Simulate performance issue and rollback

---

## Emergency Contacts

- **On-call Engineer**: [Add contact]
- **Database Administrator**: [Add contact]
- **Infrastructure Team**: [Add contact]

---

**Status**: Active  
**Last Review**: January 15, 2026
