# Pre-Roadmap Fixes Plan

## Overview
This document outlines the fixes needed before proceeding with the roadmap:
1. Fix Grafana dashboard charts that show errors
2. Fix Electricity Timeseries page (empty data)
3. Organize and clean up MD and SH files

## Task 1: Fix Grafana Dashboard Errors

### Issues Found:
1. **System Health Dashboard**: "Application Services Up" panel shows "No data"
   - Query: `count(up{job=~"airflow|nginx|redis"} == 1)`
   - Issue: Prometheus may not have these jobs, or services aren't exposing metrics correctly

### Actions:
- [ ] Check Prometheus targets status
- [ ] Fix "Application Services Up" query to use correct job names or container labels
- [ ] Review all 9 dashboards for errors:
  - [ ] system-health.json
  - [ ] data-quality.json
  - [ ] database-performance.json
  - [ ] energy-metrics.json
  - [ ] energy-osm-features.json
  - [ ] error-tracking.json
  - [ ] pipeline-status.json
  - [ ] sla-monitoring.json
  - [ ] web-analytics.json

## Task 2: Fix Electricity Timeseries Page

### Issue:
- Frontend shows empty data when calling `/api/v1/electricity/timeseries`
- Table `fact_electricity_timeseries` may be empty

### Actions:
- [ ] Check if `fact_electricity_timeseries` table exists
- [ ] Check if table has data
- [ ] Verify ENTSO-E ingestion DAG has run successfully
- [ ] Check API endpoint is working correctly
- [ ] Verify frontend is calling the correct endpoint

## Task 3: Organize MD and SH Files

### Current State:
- 159 MD files scattered across the project
- 42 SH files in various locations
- Many outdated/irrelevant files

### Proposed Structure:
```
docs/
  ├── guides/
  │   ├── user-guides/
  │   │   ├── API-EXAMPLES.md
  │   │   ├── AUTHENTICATION-GUIDE.md
  │   │   ├── DASHBOARD-TUTORIALS.md
  │   │   ├── DATA-INTERPRETATION-GUIDE.md
  │   │   ├── TROUBLESHOOTING-GUIDE.md
  │   │   ├── USER-GUIDE-API.md
  │   │   └── USER-GUIDE-DASHBOARDS.md
  │   ├── deployment/
  │   │   ├── DEPLOYMENT-GUIDE.md
  │   │   ├── DEPLOYMENT-GUIDE-SIMPLE.md
  │   │   ├── PRODUCTION-DEPLOYMENT-CHECKLIST.md
  │   │   ├── BLUE-GREEN-DEPLOYMENT-GUIDE.md
  │   │   ├── CANARY-RELEASE-GUIDE.md
  │   │   └── ROLLBACK-PROCEDURES.md
  │   └── testing/
  │       ├── FAILOVER-TESTING-GUIDE.md
  │       ├── DISASTER-RECOVERY-TESTING.md
  │       ├── STRESS-TESTING-GUIDE.md
  │       └── TESTING-CHECKLIST.md
  ├── runbooks/
  │   ├── DAG-FAILURE-RECOVERY.md
  │   ├── SERVICE-RESTART-PROCEDURES.md
  │   ├── DATABASE-MAINTENANCE.md
  │   ├── DATA-QUALITY-ISSUES.md
  │   ├── PERFORMANCE-TROUBLESHOOTING.md
  │   ├── INCIDENT-RESPONSE.md
  │   └── BACKUP-RECOVERY.md
  ├── phase-reports/
  │   ├── PHASE-4-COMPLETE.md
  │   ├── PHASE-5-COMPLETE.md
  │   ├── PHASE-6.2-COMPLETE.md
  │   ├── PHASE-6.3-COMPLETE.md
  │   ├── PHASE-6.4-COMPLETE.md
  │   └── PHASE-6.5-COMPLETE.md
  ├── fixes/
  │   ├── GRAFANA-DAG-FIXES.md
  │   ├── GRAFANA-DATASOURCE-FIX.md
  │   ├── OSM-DAG-SUCCESS.md
  │   ├── LOGIN-FIX-SUCCESS.md
  │   └── ... (other fix documents)
  └── status/
      ├── PRODUCTION-READINESS-REPORT.md
      ├── SECURITY-AUDIT-RESULTS.md
      └── ... (status documents)

scripts/
  ├── deployment/
  │   ├── deploy-energy-service.sh
  │   ├── deploy-frontend.sh
  │   ├── build-and-deploy.sh
  │   └── ... (deployment scripts)
  ├── testing/
  │   ├── test-api-endpoints.sh
  │   ├── complete-e2e-test.sh
  │   └── ... (testing scripts)
  ├── monitoring/
  │   ├── analyze-db-performance.sh
  │   ├── analyze-dag-performance.sh
  │   ├── monitor-data-quality.sh
  │   └── ... (monitoring scripts)
  ├── maintenance/
  │   ├── backup-database.sh
  │   ├── security-audit.sh
  │   ├── vulnerability-scan.sh
  │   └── ... (maintenance scripts)
  └── keycloak-setup/
      └── ... (Keycloak scripts)
```

### Actions:
- [ ] Create `docs/` directory structure
- [ ] Move relevant MD files to appropriate folders
- [ ] Move relevant SH files to appropriate folders
- [ ] Delete outdated/irrelevant files
- [ ] Update any references to moved files
- [ ] Create README.md in each docs/ subdirectory
