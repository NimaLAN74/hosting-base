# Data Ingestion Testing Guide

**Version**: 1.0  
**Date**: January 7, 2026

---

## Testing Strategy

### Phase 1: Test DAG (Small Scale)

A test DAG (`eurostat_ingestion_test`) has been created to validate the pipeline with minimal data:
- **Country**: Sweden (SE)
- **Year**: 2022
- **Table**: nrg_bal_s
- **Sample Size**: First 10 records

**Purpose**: Verify dimension decoding, transformation, and database loading work correctly.

### Phase 2: Full DAG (Production Scale)

Once test DAG succeeds, enable and run the full `eurostat_ingestion` DAG:
- **Tables**: 4 priority tables
- **Countries**: 27 EU countries
- **Years**: 2020-2023
- **Total Tasks**: ~432 tasks

---

## Test DAG Usage

### 1. Enable Test DAG

```bash
# Via Airflow CLI
docker compose -f docker-compose.airflow.yaml exec airflow-scheduler \
  airflow dags unpause eurostat_ingestion_test

# Or via Airflow UI: Admin → DAGs → eurostat_ingestion_test → Toggle On
```

### 2. Trigger Test DAG

```bash
# Via Airflow CLI
docker compose -f docker-compose.airflow.yaml exec airflow-scheduler \
  airflow dags trigger eurostat_ingestion_test

# Or via Airflow UI: Click "Play" button on DAG
```

### 3. Monitor Execution

```bash
# Check DAG runs
docker compose -f docker-compose.airflow.yaml exec airflow-scheduler \
  airflow dags list-runs -d eurostat_ingestion_test --state running

# Check task logs
docker compose -f docker-compose.airflow.yaml exec airflow-scheduler \
  airflow tasks logs eurostat_ingestion_test test_fetch 2026-01-07
```

### 4. Verify Data

```sql
-- Check ingested records
SELECT 
    country_code,
    year,
    COUNT(*) as records,
    COUNT(DISTINCT product_code) as products,
    COUNT(DISTINCT flow_code) as flows,
    MIN(value_gwh) as min_value,
    MAX(value_gwh) as max_value
FROM fact_energy_annual
WHERE source_system = 'eurostat'
  AND source_table = 'nrg_bal_s'
  AND country_code = 'SE'
  AND year = 2022
GROUP BY country_code, year;

-- Check sample records
SELECT 
    product_code,
    flow_code,
    value_gwh,
    unit,
    ingestion_timestamp
FROM fact_energy_annual
WHERE source_system = 'eurostat'
  AND country_code = 'SE'
  AND year = 2022
LIMIT 10;
```

---

## Full DAG Usage

### 1. Enable Full DAG

```bash
docker compose -f docker-compose.airflow.yaml exec airflow-scheduler \
  airflow dags unpause eurostat_ingestion
```

### 2. Trigger Manually (or wait for schedule)

```bash
docker compose -f docker-compose.airflow.yaml exec airflow-scheduler \
  airflow dags trigger eurostat_ingestion
```

### 3. Monitor Progress

```bash
# Check overall status
docker compose -f docker-compose.airflow.yaml exec airflow-scheduler \
  airflow dags list-runs -d eurostat_ingestion --state running

# Check specific task group
docker compose -f docker-compose.airflow.yaml exec airflow-scheduler \
  airflow tasks list eurostat_ingestion ingest_nrg_bal_s
```

### 4. Verify Results

```sql
-- Summary by table
SELECT 
    source_table,
    COUNT(*) as total_records,
    COUNT(DISTINCT country_code) as countries,
    COUNT(DISTINCT year) as years,
    MIN(year) as earliest_year,
    MAX(year) as latest_year
FROM fact_energy_annual
WHERE source_system = 'eurostat'
GROUP BY source_table
ORDER BY source_table;

-- Check ingestion log
SELECT 
    table_name,
    status,
    records_inserted,
    ingestion_timestamp
FROM meta_ingestion_log
WHERE source_system = 'eurostat'
ORDER BY ingestion_timestamp DESC;
```

---

## Troubleshooting

### Issue: DAG not appearing in Airflow UI

**Solution**:
1. Check DAG file syntax: `python3 -m py_compile dags/eurostat_ingestion_dag.py`
2. Check Airflow scheduler logs: `docker compose logs airflow-scheduler`
3. Wait 30-60 seconds for Airflow to refresh DAGs
4. Restart scheduler if needed: `docker compose restart airflow-scheduler`

### Issue: API Request Failures

**Symptoms**: Tasks failing with `requests.RequestException`

**Solution**:
1. Check internet connectivity from Airflow container
2. Verify Eurostat API is accessible: `curl https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/nrg_bal_s?format=JSON&geo=SE&time=2022`
3. Check rate limiting (0.5s delay between calls)
4. Review task logs for specific error messages

### Issue: Dimension Decoding Failures

**Symptoms**: Records with `NULL` product_code or flow_code

**Solution**:
1. Verify dimension structure in API response
2. Check `decode_eurostat_dimension()` function logic
3. Review sample API response to understand dimension structure
4. Test with known good data (Sweden 2022)

### Issue: Database Insert Failures

**Symptoms**: Tasks failing with `psycopg2` errors

**Solution**:
1. Verify database connection: `psql -U postgres -d lianel_energy -c "SELECT 1;"`
2. Check foreign key constraints (product_code, flow_code must exist in dimension tables)
3. Verify unique constraint exists: `\d fact_energy_annual`
4. Check for data type mismatches

### Issue: Foreign Key Violations

**Symptoms**: `violates foreign key constraint`

**Solution**:
1. Ensure dimension tables are populated:
   ```sql
   SELECT COUNT(*) FROM dim_energy_product;
   SELECT COUNT(*) FROM dim_energy_flow;
   ```
2. Load missing product/flow codes to dimension tables
3. Or update DAG to handle missing codes gracefully

---

## Expected Results

### Test DAG
- **Duration**: ~30-60 seconds
- **Records**: ~10 sample records
- **Status**: All tasks successful

### Full DAG
- **Duration**: ~2-4 hours (depending on API response times)
- **Records**: ~50,000-100,000 records per table
- **Status**: Most tasks successful (some may fail due to missing data)

---

## Next Steps After Testing

1. **Review Results**: Check data quality and completeness
2. **Fix Issues**: Address any failures or data quality problems
3. **Expand Years**: Add more years (1990-2019) once validated
4. **Implement Harmonization**: Create DAG for unit conversion
5. **Add Quality Checks**: Implement data quality validation DAG

---

## Verification Queries

### Data Completeness

```sql
-- Check coverage by country/year
SELECT 
    country_code,
    year,
    source_table,
    COUNT(*) as records
FROM fact_energy_annual
WHERE source_system = 'eurostat'
GROUP BY country_code, year, source_table
ORDER BY country_code, year, source_table;
```

### Data Quality

```sql
-- Check for NULL values
SELECT 
    source_table,
    COUNT(*) FILTER (WHERE product_code IS NULL) as null_products,
    COUNT(*) FILTER (WHERE flow_code IS NULL) as null_flows,
    COUNT(*) FILTER (WHERE value_gwh IS NULL) as null_values,
    COUNT(*) as total_records
FROM fact_energy_annual
WHERE source_system = 'eurostat'
GROUP BY source_table;
```

### Ingestion Summary

```sql
-- Overall ingestion status
SELECT 
    source_table,
    COUNT(*) as records,
    COUNT(DISTINCT country_code) as countries,
    COUNT(DISTINCT year) as years,
    MIN(ingestion_timestamp) as first_ingestion,
    MAX(ingestion_timestamp) as last_ingestion
FROM fact_energy_annual
WHERE source_system = 'eurostat'
GROUP BY source_table;
```

---

**Status**: Ready for testing. Start with test DAG, then proceed to full DAG.

