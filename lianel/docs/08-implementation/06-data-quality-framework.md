# Data Quality Framework

**Version**: 1.0  
**Date**: January 7, 2026  
**Status**: ✅ Framework Complete

---

## Executive Summary

This document defines the data quality framework for the EU Energy & Geospatial Intelligence Platform, including validation rules, quality thresholds, reconciliation procedures, and anomaly detection strategies.

**Purpose**: Ensure data reliability, completeness, and accuracy throughout the data pipeline.

---

## Quality Dimensions

### 1. Completeness

**Definition**: Percentage of expected data that is present.

**Metrics**:
- Missing records (countries, years, products)
- NULL values in required fields
- Empty API responses

**Thresholds**:
- **Critical**: <95% completeness → FAIL
- **Warning**: 95-98% completeness → WARN
- **Acceptable**: >98% completeness → PASS

---

### 2. Validity

**Definition**: Data conforms to expected formats, types, and ranges.

**Metrics**:
- Data type validation
- Value range checks
- Format validation (country codes, dates)
- Foreign key integrity

**Thresholds**:
- **Critical**: Any invalid required field → FAIL
- **Warning**: Invalid optional fields >5% → WARN
- **Acceptable**: All required fields valid → PASS

---

### 3. Consistency

**Definition**: Data is consistent across sources and time periods.

**Metrics**:
- Cross-table consistency
- Temporal consistency (year-over-year changes)
- Aggregation consistency (totals match sums)

**Thresholds**:
- **Critical**: Major inconsistencies (>10% difference) → FAIL
- **Warning**: Minor inconsistencies (5-10% difference) → WARN
- **Acceptable**: <5% difference → PASS

---

### 4. Accuracy

**Definition**: Data correctly represents real-world values.

**Metrics**:
- Comparison with known benchmarks
- Statistical validation
- Outlier detection

**Thresholds**:
- **Critical**: Systematic errors detected → FAIL
- **Warning**: Outliers >5% of data → WARN
- **Acceptable**: Outliers <5% → PASS

---

### 5. Timeliness

**Definition**: Data is available within expected timeframes.

**Metrics**:
- Ingestion delay (time from source update to ingestion)
- Processing delay (time from ingestion to availability)

**Thresholds**:
- **Critical**: Delay >7 days → FAIL
- **Warning**: Delay 3-7 days → WARN
- **Acceptable**: Delay <3 days → PASS

---

## Validation Rules by Table

### fact_energy_annual

#### Completeness Rules

1. **Country Coverage**:
   ```sql
   -- Check: All EU27 countries should have data for each year
   SELECT 
       year,
       COUNT(DISTINCT country_code) as countries_with_data,
       27 as expected_countries,
       ROUND(100.0 * COUNT(DISTINCT country_code) / 27, 2) as completeness_pct
   FROM fact_energy_annual
   WHERE year >= 2020
   GROUP BY year
   HAVING COUNT(DISTINCT country_code) < 27;
   ```
   - **Threshold**: <27 countries → WARN, <25 countries → FAIL

2. **Year Coverage**:
   ```sql
   -- Check: Each country should have data for all expected years
   SELECT 
       country_code,
       COUNT(DISTINCT year) as years_with_data,
       (2023 - 1990 + 1) as expected_years,
       ROUND(100.0 * COUNT(DISTINCT year) / (2023 - 1990 + 1), 2) as completeness_pct
   FROM fact_energy_annual
   WHERE country_code IN (SELECT country_code FROM dim_country WHERE eu_member = true)
   GROUP BY country_code
   HAVING COUNT(DISTINCT year) < (2023 - 1990 + 1) * 0.95;
   ```
   - **Threshold**: <95% years → WARN, <90% years → FAIL

3. **NULL Values**:
   ```sql
   -- Check: Required fields should not be NULL
   SELECT 
       COUNT(*) FILTER (WHERE country_code IS NULL) as null_country_code,
       COUNT(*) FILTER (WHERE year IS NULL) as null_year,
       COUNT(*) FILTER (WHERE value_gwh IS NULL) as null_value,
       COUNT(*) as total_rows
   FROM fact_energy_annual
   WHERE year >= 2020;
   ```
   - **Threshold**: Any NULL in required fields → FAIL

---

#### Validity Rules

1. **Value Ranges**:
   ```sql
   -- Check: Energy values should be non-negative and reasonable
   SELECT 
       COUNT(*) FILTER (WHERE value_gwh < 0) as negative_values,
       COUNT(*) FILTER (WHERE value_gwh > 1000000) as extreme_values,
       COUNT(*) as total_rows
   FROM fact_energy_annual
   WHERE year >= 2020;
   ```
   - **Threshold**: Any negative values → FAIL
   - **Threshold**: >1% extreme values (>1M GWh) → WARN

2. **Year Range**:
   ```sql
   -- Check: Years should be within valid range
   SELECT COUNT(*) as invalid_years
   FROM fact_energy_annual
   WHERE year < 1960 OR year > 2100;
   ```
   - **Threshold**: Any invalid years → FAIL

3. **Country Code Format**:
   ```sql
   -- Check: Country codes should be valid ISO 3166-1 alpha-2
   SELECT DISTINCT country_code
   FROM fact_energy_annual
   WHERE LENGTH(country_code) != 2
      OR country_code !~ '^[A-Z]{2}$';
   ```
   - **Threshold**: Any invalid format → FAIL

4. **Foreign Key Integrity**:
   ```sql
   -- Check: All country codes exist in dim_country
   SELECT DISTINCT e.country_code
   FROM fact_energy_annual e
   LEFT JOIN dim_country c ON e.country_code = c.country_code
   WHERE c.country_code IS NULL;
   ```
   - **Threshold**: Any missing foreign keys → FAIL

---

#### Consistency Rules

1. **Year-over-Year Changes**:
   ```sql
   -- Check: Year-over-year changes should be reasonable (<50% change)
   WITH year_comparison AS (
       SELECT 
           country_code,
           product_code,
           flow_code,
           year,
           value_gwh,
           LAG(value_gwh) OVER (
               PARTITION BY country_code, product_code, flow_code 
               ORDER BY year
           ) as prev_year_value
       FROM fact_energy_annual
       WHERE year >= 2020
   )
   SELECT 
       COUNT(*) as extreme_changes,
       COUNT(*) FILTER (WHERE prev_year_value > 0 
                        AND ABS(value_gwh - prev_year_value) / prev_year_value > 0.5) as changes_over_50pct
   FROM year_comparison
   WHERE prev_year_value IS NOT NULL;
   ```
   - **Threshold**: >5% records with >50% change → WARN
   - **Threshold**: >10% records with >50% change → FAIL

2. **Unit Consistency**:
   ```sql
   -- Check: All values should be in GWh after harmonization
   SELECT DISTINCT unit
   FROM fact_energy_annual
   WHERE unit != 'GWh';
   ```
   - **Threshold**: Any non-GWh units → FAIL (after harmonization)

---

### dim_region

#### Completeness Rules

1. **NUTS Level Coverage**:
   ```sql
   -- Check: All NUTS levels should have data
   SELECT 
       level_code,
       COUNT(*) as region_count,
       CASE level_code
           WHEN 0 THEN 37  -- Expected NUTS0 regions
           WHEN 1 THEN 125 -- Expected NUTS1 regions
           WHEN 2 THEN 334 -- Expected NUTS2 regions
       END as expected_count
   FROM dim_region
   GROUP BY level_code;
   ```
   - **Threshold**: <90% of expected regions → WARN

2. **Geometry Completeness**:
   ```sql
   -- Check: All regions should have geometries
   SELECT 
       COUNT(*) FILTER (WHERE geometry IS NULL) as missing_geometry,
       COUNT(*) FILTER (WHERE geometry_wgs84 IS NULL) as missing_wgs84,
       COUNT(*) as total_regions
   FROM dim_region;
   ```
   - **Threshold**: Any missing geometries → FAIL

---

#### Validity Rules

1. **Geometry Validity**:
   ```sql
   -- Check: All geometries should be valid
   SELECT 
       region_id,
       ST_IsValid(geometry) as geometry_valid,
       ST_IsValid(geometry_wgs84) as wgs84_valid
   FROM dim_region
   WHERE NOT ST_IsValid(geometry) OR NOT ST_IsValid(geometry_wgs84);
   ```
   - **Threshold**: Any invalid geometries → FAIL

2. **Area Calculation**:
   ```sql
   -- Check: Calculated area should match geometry area
   SELECT 
       region_id,
       area_km2,
       ST_Area(geometry) / 1000000 as calculated_area_km2,
       ABS(area_km2 - ST_Area(geometry) / 1000000) as difference
   FROM dim_region
   WHERE ABS(area_km2 - ST_Area(geometry) / 1000000) > 1;  -- >1 km² difference
   ```
   - **Threshold**: >1 km² difference → WARN

---

### dim_country

#### Validity Rules

1. **Country Code Uniqueness**:
   ```sql
   -- Check: Country codes should be unique
   SELECT country_code, COUNT(*) as count
   FROM dim_country
   GROUP BY country_code
   HAVING COUNT(*) > 1;
   ```
   - **Threshold**: Any duplicates → FAIL

2. **EU27 Coverage**:
   ```sql
   -- Check: All EU27 countries should be present
   SELECT COUNT(*) as eu_countries
   FROM dim_country
   WHERE eu_member = true;
   ```
   - **Threshold**: <27 EU countries → FAIL

---

## Anomaly Detection

### Statistical Outliers

**Method**: Z-score analysis

```sql
-- Detect outliers using Z-score (>3 standard deviations)
WITH stats AS (
    SELECT 
        AVG(value_gwh) as mean_value,
        STDDEV(value_gwh) as stddev_value
    FROM fact_energy_annual
    WHERE year >= 2020
      AND product_code = 'C0000'  -- Total energy
      AND flow_code = 'FC'         -- Final consumption
)
SELECT 
    country_code,
    year,
    value_gwh,
    ABS(value_gwh - mean_value) / stddev_value as z_score
FROM fact_energy_annual, stats
WHERE year >= 2020
  AND product_code = 'C0000'
  AND flow_code = 'FC'
  AND ABS(value_gwh - mean_value) / stddev_value > 3;
```

**Threshold**: Z-score >3 → Flag for review

---

### Temporal Anomalies

**Method**: Detect sudden changes or missing periods

```sql
-- Detect missing years (gaps in time series)
WITH year_series AS (
    SELECT generate_series(1990, 2023) as expected_year
),
country_years AS (
    SELECT DISTINCT country_code, year
    FROM fact_energy_annual
)
SELECT 
    c.country_code,
    y.expected_year,
    CASE WHEN cy.year IS NULL THEN 'MISSING' ELSE 'PRESENT' END as status
FROM dim_country c
CROSS JOIN year_series y
LEFT JOIN country_years cy ON c.country_code = cy.country_code 
                            AND y.expected_year = cy.year
WHERE c.eu_member = true
  AND cy.year IS NULL
ORDER BY c.country_code, y.expected_year;
```

**Threshold**: Missing years >2 consecutive → WARN

---

### Cross-Source Validation

**Method**: Compare with external sources (if available)

```sql
-- Example: Compare Eurostat totals with known benchmarks
-- (This would require external reference data)
SELECT 
    country_code,
    year,
    SUM(value_gwh) as total_energy_gwh,
    -- Compare with known country totals (if available)
    CASE 
        WHEN country_code = 'SE' AND year = 2022 
        THEN ABS(SUM(value_gwh) - 500000)  -- Example benchmark
        ELSE 0
    END as difference_from_benchmark
FROM fact_energy_annual
WHERE year = 2022
  AND flow_code = 'FC'
GROUP BY country_code, year
HAVING ABS(SUM(value_gwh) - expected_benchmark) / expected_benchmark > 0.1;
```

**Threshold**: >10% difference from benchmark → WARN

---

## Quality Check Procedures

### Automated Checks (Daily)

**DAG**: `data_quality_check`

**Checks Performed**:
1. Completeness checks (country/year coverage)
2. Validity checks (data types, ranges, formats)
3. Foreign key integrity
4. NULL value checks
5. Basic consistency checks

**Results**: Stored in `meta_data_quality` table

---

### Manual Review Triggers

**Automatic Flags**:
- Quality check result = 'fail'
- Anomaly detected (Z-score >3)
- Missing data >5%
- Consistency issues >10%

**Review Process**:
1. Investigate flagged records
2. Compare with source data
3. Document findings
4. Update data if needed
5. Update quality check results

---

## Reconciliation Procedures

### Source Reconciliation

**Purpose**: Verify ingested data matches source

**Procedure**:
1. Re-fetch sample records from Eurostat API
2. Compare with database records
3. Verify values match (within rounding tolerance)
4. Document any discrepancies

**Frequency**: Monthly (sample-based)

---

### Cross-Table Reconciliation

**Purpose**: Verify consistency across related tables

**Example**: Verify country totals match sum of regions
```sql
-- This would require region-level data (future)
SELECT 
    c.country_code,
    SUM(r.regional_total) as sum_of_regions,
    c.country_total,
    ABS(SUM(r.regional_total) - c.country_total) as difference
FROM country_totals c
JOIN region_totals r ON c.country_code = r.country_code
GROUP BY c.country_code, c.country_total
HAVING ABS(SUM(r.regional_total) - c.country_total) > 1000;  -- >1 TWh difference
```

---

## Quality Metrics Dashboard

### Key Metrics to Display

1. **Overall Quality Score**:
   - Weighted average of all quality dimensions
   - Trend over time

2. **Completeness by Dimension**:
   - Country coverage
   - Year coverage
   - Product/flow coverage

3. **Quality Issues**:
   - Failed checks count
   - Warning count
   - Anomalies detected

4. **Data Freshness**:
   - Last ingestion date
   - Days since last update
   - Ingestion success rate

---

## Quality Thresholds Summary

| Quality Dimension | Metric | Warning | Fail |
|-------------------|--------|---------|------|
| **Completeness** | Country coverage | <27 countries | <25 countries |
| | Year coverage | <95% | <90% |
| | NULL values | >2% | >5% |
| **Validity** | Invalid formats | >1% | Any required field |
| | Value ranges | >1% outliers | Any negative |
| | Foreign keys | >1% missing | Any missing |
| **Consistency** | Year-over-year | >5% >50% change | >10% >50% change |
| | Unit consistency | Any non-GWh | Any non-GWh |
| **Timeliness** | Ingestion delay | 3-7 days | >7 days |

---

## Implementation in Airflow

### Quality Check Task

```python
def run_quality_checks(**context):
    from airflow.providers.postgres.hooks.postgres import PostgresHook
    
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    checks = [
        ('completeness_country', check_country_coverage),
        ('completeness_year', check_year_coverage),
        ('validity_ranges', check_value_ranges),
        ('validity_foreign_keys', check_foreign_keys),
        ('consistency_yoy', check_year_over_year),
    ]
    
    results = []
    for check_name, check_func in checks:
        try:
            result = check_func(db_hook)
            results.append({
                'quality_check': check_name,
                'result': result['status'],
                'details': result['details']
            })
        except Exception as e:
            results.append({
                'quality_check': check_name,
                'result': 'fail',
                'details': {'error': str(e)}
            })
    
    # Store results
    for result in results:
        db_hook.run("""
            INSERT INTO meta_data_quality 
            (table_name, quality_check, result, details, check_timestamp)
            VALUES ('fact_energy_annual', %s, %s, %s::jsonb, NOW())
        """, parameters=(
            result['quality_check'],
            result['result'],
            json.dumps(result['details'])
        ))
    
    # Determine overall status
    if any(r['result'] == 'fail' for r in results):
        raise AirflowException('Quality checks failed')
    elif any(r['result'] == 'warn' for r in results):
        context['ti'].xcom_push(key='quality_status', value='warn')
    else:
        context['ti'].xcom_push(key='quality_status', value='pass')
```

---

## Alerting Rules

### Critical Alerts (Immediate Notification)

1. **Data Quality Failure**:
   - Any quality check result = 'fail'
   - Action: Notify data team, pause ingestion

2. **Missing Critical Data**:
   - >10% missing countries/years
   - Action: Investigate source issues

3. **Data Corruption**:
   - Invalid geometries
   - Foreign key violations
   - Action: Immediate investigation

---

### Warning Alerts (Daily Summary)

1. **Quality Warnings**:
   - Any quality check result = 'warn'
   - Action: Review in daily summary

2. **Anomalies Detected**:
   - Statistical outliers >5%
   - Action: Flag for review

3. **Ingestion Delays**:
   - Delay >3 days
   - Action: Monitor and investigate

---

## Data Quality Reports

### Daily Report

**Contents**:
- Overall quality score
- Failed checks summary
- Warning summary
- Data freshness status

**Delivery**: Email/Slack (if configured)

---

### Weekly Report

**Contents**:
- Quality trends (7-day)
- Top quality issues
- Reconciliation results
- Recommendations

---

## Continuous Improvement

### Quality Metrics Tracking

Track quality metrics over time to identify:
- Degrading data quality trends
- Recurring issues
- Source data problems

### Feedback Loop

1. Identify quality issues
2. Investigate root causes
3. Update validation rules if needed
4. Improve data sources/processes
5. Monitor improvements

---

## References

- **Database Schema**: `04-data-model/05-schema-ddl.sql`
- **Airflow DAGs**: `08-implementation/04-airflow-dag-design.md`
- **Data Coverage**: `08-implementation/02-data-coverage-assessment.md`
- **PostgreSQL Documentation**: https://www.postgresql.org/docs/

---

**Status**: ✅ Data quality framework complete. Ready for implementation in Airflow DAGs.

