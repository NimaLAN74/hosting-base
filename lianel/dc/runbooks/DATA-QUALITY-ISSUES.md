# Data Quality Issues Runbook
**Last Updated**: January 15, 2026  
**Owner**: Operations Team

---

## Overview

This runbook provides procedures for identifying and resolving data quality issues in the Lianel platform.

---

## Quick Reference

| Issue | Detection | Resolution |
|-------|-----------|------------|
| Stale data | Data freshness >24h | Re-run ingestion DAGs |
| Missing data | Record count = 0 | Check ingestion logs |
| Incomplete data | Completeness <95% | Investigate source |
| Data anomalies | Validation failures | Review data source |

---

## 1. Data Quality Monitoring

### 1.1 Check Data Freshness

#### Procedure
```bash
# Using monitoring script
cd /root/lianel/dc
bash scripts/monitor-data-quality.sh

# Or manually
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
SELECT 
    EXTRACT(EPOCH FROM (NOW() - MAX(ingestion_date))) / 3600 as age_hours,
    MAX(ingestion_date) as last_ingestion
FROM meta_ingestion_log
WHERE ingestion_date IS NOT NULL;
EOF
```

#### Thresholds
- **Green**: <12 hours
- **Yellow**: 12-24 hours
- **Red**: >24 hours (SLA violation)

---

### 1.2 Check Data Completeness

#### Procedure
```bash
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
SELECT 
    COUNT(*) as total_records,
    COUNT(*) FILTER (WHERE country_code IS NOT NULL) as has_country,
    COUNT(*) FILTER (WHERE year IS NOT NULL) as has_year,
    COUNT(*) FILTER (WHERE value IS NOT NULL) as has_value,
    COUNT(*) FILTER (WHERE country_code IS NOT NULL AND year IS NOT NULL AND value IS NOT NULL) as complete_records,
    (COUNT(*) FILTER (WHERE country_code IS NOT NULL AND year IS NOT NULL AND value IS NOT NULL)::float / COUNT(*)::float * 100) as completeness_pct
FROM fact_energy_annual;
EOF
```

#### Thresholds
- **Green**: >95%
- **Yellow**: 90-95%
- **Red**: <90%

---

### 1.3 Check Record Counts

#### Procedure
```bash
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
-- Energy data
SELECT 'fact_energy_annual' as table_name, COUNT(*) as record_count FROM fact_energy_annual
UNION ALL
SELECT 'fact_geo_region_features', COUNT(*) FROM fact_geo_region_features
UNION ALL
SELECT 'ml_dataset_forecasting_v1', COUNT(*) FROM ml_dataset_forecasting_v1
UNION ALL
SELECT 'ml_dataset_clustering_v1', COUNT(*) FROM ml_dataset_clustering_v1
UNION ALL
SELECT 'ml_dataset_geo_enrichment_v1', COUNT(*) FROM ml_dataset_geo_enrichment_v1;
EOF
```

---

## 2. Common Data Quality Issues

### 2.1 Stale Data

#### Symptoms
- Data freshness >24 hours
- Last ingestion date is old
- Alerts firing for data freshness

#### Investigation
```bash
# Check last ingestion
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
SELECT 
    source_name,
    MAX(ingestion_date) as last_ingestion,
    EXTRACT(EPOCH FROM (NOW() - MAX(ingestion_date))) / 3600 as age_hours
FROM meta_ingestion_log
GROUP BY source_name
ORDER BY age_hours DESC;
EOF

# Check DAG status
docker exec dc-airflow-scheduler-1 airflow dags list-runs -d entsoe_ingestion --state failed --limit 5
docker exec dc-airflow-scheduler-1 airflow dags list-runs -d osm_feature_extraction --state failed --limit 5
```

#### Resolution
1. **Check DAG status**:
   - Review failed DAG runs
   - Check error logs
   - Identify root cause

2. **Re-run ingestion**:
   ```bash
   # Trigger DAG manually
   docker exec dc-airflow-scheduler-1 airflow dags trigger entsoe_ingestion
   docker exec dc-airflow-scheduler-1 airflow dags trigger osm_feature_extraction
   ```

3. **Fix root cause**:
   - Fix DAG errors
   - Resolve API issues
   - Fix database issues

4. **Verify**:
   - Check data freshness after re-run
   - Verify new data ingested
   - Monitor for recurring issues

---

### 2.2 Missing Data

#### Symptoms
- Record count = 0
- Empty tables
- Missing expected data

#### Investigation
```bash
# Check table record counts
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
SELECT 
    'fact_energy_annual' as table_name,
    COUNT(*) as record_count,
    MIN(year) as min_year,
    MAX(year) as max_year
FROM fact_energy_annual;
EOF

# Check ingestion logs
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
SELECT 
    source_name,
    COUNT(*) as ingestion_count,
    MAX(ingestion_date) as last_ingestion,
    COUNT(*) FILTER (WHERE status = 'success') as success_count,
    COUNT(*) FILTER (WHERE status = 'failed') as failed_count
FROM meta_ingestion_log
WHERE ingestion_date > NOW() - INTERVAL '30 days'
GROUP BY source_name;
EOF
```

#### Resolution
1. **Check ingestion history**:
   - Review ingestion logs
   - Check for failed ingestions
   - Identify when data stopped

2. **Check DAG runs**:
   ```bash
   # Check recent DAG runs
   docker exec dc-airflow-scheduler-1 airflow dags list-runs -d entsoe_ingestion --limit 10
   ```

3. **Check data source**:
   - Verify API access
   - Check API credentials
   - Test API endpoints

4. **Re-run ingestion**:
   - Clear failed tasks
   - Trigger DAG
   - Monitor execution

5. **Backfill if needed**:
   - Identify missing date ranges
   - Re-run for specific dates
   - Verify data loaded

---

### 2.3 Incomplete Data

#### Symptoms
- Completeness <95%
- Many NULL values
- Missing required fields

#### Investigation
```bash
# Check completeness by field
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
SELECT 
    COUNT(*) as total,
    COUNT(country_code) as has_country,
    COUNT(year) as has_year,
    COUNT(value) as has_value,
    COUNT(unit) as has_unit,
    (COUNT(country_code)::float / COUNT(*)::float * 100) as country_pct,
    (COUNT(year)::float / COUNT(*)::float * 100) as year_pct,
    (COUNT(value)::float / COUNT(*)::float * 100) as value_pct
FROM fact_energy_annual;
EOF

# Check by country
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
SELECT 
    country_code,
    COUNT(*) as total,
    COUNT(value) as has_value,
    (COUNT(value)::float / COUNT(*)::float * 100) as completeness_pct
FROM fact_energy_annual
GROUP BY country_code
HAVING (COUNT(value)::float / COUNT(*)::float * 100) < 95
ORDER BY completeness_pct ASC;
EOF
```

#### Resolution
1. **Identify patterns**:
   - Check which fields are missing
   - Identify affected countries/regions
   - Check date ranges

2. **Check data source**:
   - Verify API responses
   - Check data format
   - Review source data quality

3. **Fix data processing**:
   - Update DAG logic if needed
   - Fix data transformation
   - Add validation

4. **Re-process data**:
   - Clear incomplete records
   - Re-run ingestion
   - Verify completeness

---

### 2.4 Data Anomalies

#### Symptoms
- Validation failures
- Unexpected values
- Outliers detected
- Data format errors

#### Investigation
```bash
# Check for outliers
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
SELECT 
    country_code,
    year,
    value,
    unit,
    CASE 
        WHEN value < 0 THEN 'Negative value'
        WHEN value > 1000000 THEN 'Very large value'
        ELSE 'OK'
    END as anomaly_type
FROM fact_energy_annual
WHERE value < 0 OR value > 1000000
ORDER BY value DESC
LIMIT 20;
EOF

# Check data distribution
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
SELECT 
    country_code,
    COUNT(*) as record_count,
    MIN(value) as min_value,
    MAX(value) as max_value,
    AVG(value) as avg_value,
    PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY value) as median_value
FROM fact_energy_annual
WHERE value IS NOT NULL
GROUP BY country_code
ORDER BY country_code;
EOF
```

#### Resolution
1. **Identify anomalies**:
   - Review validation errors
   - Check for outliers
   - Identify data patterns

2. **Investigate source**:
   - Check source data
   - Verify API responses
   - Review data transformation

3. **Fix or exclude**:
   - Fix data if correctable
   - Exclude invalid records
   - Update validation rules

4. **Document**:
   - Document anomaly type
   - Update data quality rules
   - Add monitoring alerts

---

## 3. Data Quality Checks

### 3.1 Automated Checks

#### Using Monitoring Script
```bash
cd /root/lianel/dc
bash scripts/monitor-data-quality.sh
```

#### Using Grafana Dashboard
- Access: `https://monitoring.lianel.se/d/data-quality`
- Review all data quality metrics
- Check for alerts

---

### 3.2 Manual Checks

#### Daily Checks
- Data freshness
- Record counts
- Error logs

#### Weekly Checks
- Data completeness
- Data distribution
- Anomaly detection

#### Monthly Checks
- Full data quality audit
- Review data quality trends
- Update quality rules

---

## 4. Resolution Procedures

### 4.1 Quick Fixes

#### Re-run Failed Ingestion
```bash
# Check failed DAGs
docker exec dc-airflow-scheduler-1 airflow dags list-runs --state failed --limit 5

# Clear and re-run
docker exec dc-airflow-scheduler-1 airflow dags clear <dag_id> --start-date <date> --end-date <date>
docker exec dc-airflow-scheduler-1 airflow dags trigger <dag_id>
```

#### Fix Configuration Issues
```bash
# Check DAG configuration
docker exec dc-airflow-scheduler-1 airflow dags show <dag_id>

# Check environment variables
docker exec dc-airflow-scheduler-1 env | grep -E 'ENTSOE|OSM|API'
```

---

### 4.2 Data Backfill

#### Procedure
```bash
# Identify missing date range
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy <<EOF
SELECT 
    country_code,
    MIN(year) as min_year,
    MAX(year) as max_year,
    COUNT(DISTINCT year) as year_count
FROM fact_energy_annual
GROUP BY country_code;
EOF

# Re-run DAG for specific date range
# Modify DAG or use Airflow UI to set date range
docker exec dc-airflow-scheduler-1 airflow dags trigger <dag_id> --conf '{"start_date":"YYYY-MM-DD","end_date":"YYYY-MM-DD"}'
```

---

## 5. Prevention

### 5.1 Monitoring
- Set up data quality alerts
- Monitor data freshness
- Track completeness metrics
- Review anomaly detection

### 5.2 Validation
- Implement data validation in DAGs
- Add schema validation
- Check data types
- Validate ranges

### 5.3 Documentation
- Document data quality rules
- Update runbooks
- Share findings
- Track improvements

---

## 6. Escalation

### When to Escalate
- Data quality issues persist >24 hours
- Critical data missing
- Data corruption detected
- Unable to resolve after 1 hour

### Escalation Path
1. **Level 1**: Operations team
2. **Level 2**: Data engineering team
3. **Level 3**: Development team
4. **Level 4**: Management

---

## 7. Quick Commands Reference

```bash
# Check data freshness
bash scripts/monitor-data-quality.sh

# Check record counts
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy -c "SELECT COUNT(*) FROM fact_energy_annual;"

# Check completeness
PGPASSWORD=${POSTGRES_PASSWORD} psql -h 172.18.0.1 -p 5432 -U postgres -d lianel_energy -c "SELECT (COUNT(*) FILTER (WHERE country_code IS NOT NULL AND year IS NOT NULL AND value IS NOT NULL)::float / COUNT(*)::float * 100) as completeness FROM fact_energy_annual;"

# Re-run ingestion
docker exec dc-airflow-scheduler-1 airflow dags trigger entsoe_ingestion

# Check DAG status
docker exec dc-airflow-scheduler-1 airflow dags list-runs -d entsoe_ingestion --limit 5
```

---

**Status**: Active  
**Review Frequency**: Monthly  
**Last Review**: January 15, 2026
