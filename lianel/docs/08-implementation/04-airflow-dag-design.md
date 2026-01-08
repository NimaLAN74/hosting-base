good, follow your next# Airflow DAG Design

**Version**: 1.0  
**Date**: January 7, 2026  
**Status**: ✅ Design Complete

---

## Executive Summary

This document specifies the Airflow DAG architecture for the EU Energy & Geospatial Intelligence Platform, including DAG structures, task dependencies, error handling, and idempotency strategies.

**Airflow Version**: 3.1.3  
**Executor**: CeleryExecutor  
**Database**: PostgreSQL (host-level)  
**Broker**: Redis

---

## DAG Overview

### DAG List

| DAG ID | Description | Schedule | Priority |
|--------|-------------|----------|----------|
| `eurostat_ingestion` | Eurostat data ingestion | Weekly (Sunday 02:00) | High |
| `eurostat_harmonization` | Data harmonization and transformation | After ingestion | High |
| `nuts_boundary_update` | NUTS boundary updates | Annually (January) | Medium |
| `data_quality_check` | Data quality validation | Daily | High |
| `ml_dataset_generation` | ML dataset creation | Weekly (after harmonization) | Medium |

---

## DAG 1: Eurostat Ingestion

### Purpose

Ingest annual energy data from Eurostat API for all EU27 countries and priority tables.

### Schedule

- **Frequency**: Weekly (every Sunday at 02:00 UTC)
- **Catchup**: False (don't backfill automatically)
- **Timeout**: 6 hours

### DAG Structure

```python
from datetime import datetime, timedelta
from airflow import DAG
from airflow.operators.python import PythonOperator
from airflow.operators.bash import BashOperator
from airflow.utils.task_group import TaskGroup

default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'start_date': datetime(2026, 1, 1),
    'email_on_failure': False,
    'email_on_retry': False,
    'retries': 3,
    'retry_delay': timedelta(minutes=10),
    'execution_timeout': timedelta(hours=6),
}

dag = DAG(
    'eurostat_ingestion',
    default_args=default_args,
    description='Ingest Eurostat energy data for EU27 countries',
    schedule='0 2 * * 0',  # Every Sunday at 02:00 UTC
    catchup=False,
    tags=['data-ingestion', 'eurostat', 'energy'],
    max_active_runs=1,
)
```

### Task Flow

```
start
  ↓
validate_connections
  ↓
check_last_ingestion
  ↓
┌─────────────────────────────────────┐
│  For each table (nrg_bal_s, etc.)  │
│  ┌───────────────────────────────┐ │
│  │ fetch_table_data              │ │
│  │   ↓                           │ │
│  │ validate_response             │ │
│  │   ↓                           │ │
│  │ transform_to_staging          │ │
│  │   ↓                           │ │
│  │ load_to_staging                │ │
│  └───────────────────────────────┘ │
└─────────────────────────────────────┘
  ↓
log_ingestion_summary
  ↓
end
```

### Tasks

#### 1. validate_connections

**Type**: PythonOperator  
**Purpose**: Validate database and API connections

**Logic**:
- Test PostgreSQL connection
- Test Eurostat API accessibility
- Verify required tables exist

**Failure**: Fail DAG if connections invalid

---

#### 2. check_last_ingestion

**Type**: PythonOperator  
**Purpose**: Check last successful ingestion date

**Logic**:
- Query `meta_ingestion_log` for last success
- Determine which years need ingestion
- Set XCom with ingestion plan

**Output**: XCom with `{'years': [2020, 2021, 2022, 2023], 'tables': ['nrg_bal_s', ...]}`

---

#### 3. fetch_table_data (Dynamic Task Group)

**Type**: PythonOperator (within TaskGroup)  
**Purpose**: Fetch data from Eurostat API

**Parameters**:
- `table_code`: Eurostat table code (e.g., 'nrg_bal_s')
- `country_code`: ISO 2-letter country code
- `year`: Year to fetch

**Logic**:
```python
def fetch_eurostat_data(table_code, country_code, year, **context):
    import requests
    from airflow.exceptions import AirflowException
    
    base_url = 'https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data'
    params = {
        'format': 'JSON',
        'geo': country_code,
        'time': str(year)
    }
    
    try:
        response = requests.get(
            f'{base_url}/{table_code}',
            params=params,
            timeout=60
        )
        response.raise_for_status()
        
        data = response.json()
        
        # Validate response structure
        if 'value' not in data or not data['value']:
            raise AirflowException(f'No data returned for {country_code} {year}')
        
        # Store in XCom for next task
        return {
            'data': data,
            'record_count': len(data['value'])
        }
    except requests.RequestException as e:
        raise AirflowException(f'API request failed: {e}')
```

**Error Handling**:
- Retry on network errors (3 retries, 10 min delay)
- Fail on API errors (4xx, 5xx)
- Log errors to `meta_ingestion_log`

---

#### 4. validate_response

**Type**: PythonOperator  
**Purpose**: Validate API response structure

**Logic**:
- Check required fields exist
- Validate data types
- Check for empty responses
- Validate dimension structure

**Failure**: Mark task as failed, log to metadata

---

#### 5. transform_to_staging

**Type**: PythonOperator  
**Purpose**: Transform Eurostat JSON to staging format

**Logic**:
```python
def transform_eurostat_data(table_code, country_code, year, **context):
    # Get data from previous task
    ti = context['ti']
    api_data = ti.xcom_pull(task_ids=f'fetch_{table_code}_{country_code}_{year}')
    
    # Transform Eurostat JSON structure to flat records
    records = []
    dimensions = api_data['data']['dimension']
    values = api_data['data']['value']
    
    # Decode dimension indices to values
    for key, value in values.items():
        record = {
            'country_code': country_code,
            'year': year,
            'product_code': decode_dimension(dimensions, 'siec', key),
            'flow_code': decode_dimension(dimensions, 'nrg_bal', key),
            'value_gwh': value,  # Will be harmonized later
            'source_table': table_code,
            'source_system': 'eurostat',
        }
        records.append(record)
    
    return records
```

**Output**: List of records ready for staging

---

#### 6. load_to_staging

**Type**: PythonOperator  
**Purpose**: Load transformed data to staging table

**Logic**:
- Use PostgreSQL hook to insert records
- Use bulk insert for performance
- Handle duplicates (ON CONFLICT DO NOTHING)
- Track inserted/updated counts

**Idempotency**: 
- Use `(country_code, year, product_code, flow_code, source_table)` as unique key
- ON CONFLICT DO UPDATE for updates

---

#### 7. log_ingestion_summary

**Type**: PythonOperator  
**Purpose**: Log ingestion results to metadata table

**Logic**:
- Aggregate results from all tasks
- Insert into `meta_ingestion_log`
- Calculate totals (inserted, updated, failed)
- Set status (success, partial, failed)

---

### Error Handling Strategy

1. **API Failures**:
   - Retry 3 times with 10-minute delay
   - Log to `meta_ingestion_log` with error details
   - Continue with other countries/tables if one fails

2. **Database Failures**:
   - Retry 3 times
   - Fail DAG if database unavailable

3. **Data Quality Issues**:
   - Log warnings to `meta_data_quality`
   - Continue ingestion (don't fail)
   - Flag for manual review

4. **Partial Failures**:
   - Mark ingestion as 'partial' in metadata
   - Log which countries/tables failed
   - Allow manual retry of failed tasks

---

### Idempotency

1. **Task Level**:
   - Each task checks if data already exists
   - Skip if data already ingested (same country/year/table)

2. **Data Level**:
   - Use unique constraints in database
   - ON CONFLICT DO UPDATE for updates
   - Track ingestion timestamp

3. **DAG Level**:
   - `catchup=False` prevents backfilling
   - Manual trigger for specific years if needed

---

## DAG 2: Eurostat Harmonization

### Purpose

Transform and harmonize raw Eurostat data:
- Convert units to GWh
- Normalize country codes
- Map product/flow codes
- Calculate derived metrics

### Schedule

- **Frequency**: After `eurostat_ingestion` completes
- **Trigger**: External trigger from ingestion DAG
- **Timeout**: 4 hours

### DAG Structure

```python
dag = DAG(
    'eurostat_harmonization',
    default_args=default_args,
    description='Harmonize and transform Eurostat data',
    schedule=None,  # Triggered externally
    catchup=False,
    tags=['data-transformation', 'harmonization'],
)
```

### Task Flow

```
start
  ↓
validate_staging_data
  ↓
convert_units_to_gwh
  ↓
normalize_country_codes
  ↓
map_product_codes
  ↓
map_flow_codes
  ↓
calculate_derived_metrics
  ↓
load_to_fact_table
  ↓
validate_harmonized_data
  ↓
end
```

### Key Tasks

#### convert_units_to_gwh

**Purpose**: Convert all units to GWh (harmonization standard)

**Logic**:
- Read from staging (various units: KTOE, TJ, GWh)
- Apply conversion factors:
  - KTOE → GWh: multiply by 11.63
  - TJ → GWh: multiply by 0.2778
  - GWh → GWh: no change
- Update `value_gwh` and `unit` fields

**Idempotency**: Check if already harmonized (by `harmonisation_version`)

---

#### load_to_fact_table

**Purpose**: Load harmonized data to `fact_energy_annual`

**Logic**:
- Insert into partitioned `fact_energy_annual` table
- Handle partition creation (if year doesn't exist)
- Use bulk insert for performance
- Track ingestion in metadata

---

## DAG 3: NUTS Boundary Update

### Purpose

Download and load NUTS geospatial boundaries from GISCO.

### Schedule

- **Frequency**: Annually (first Sunday of January)
- **Cron**: `0 3 1-7 1 0` (First Sunday of January at 03:00 UTC)
- **Timeout**: 2 hours

### DAG Structure

```python
dag = DAG(
    'nuts_boundary_update',
    default_args=default_args,
    description='Update NUTS geospatial boundaries from GISCO',
    schedule='0 3 1-7 1 0',  # First Sunday of January
    catchup=False,
    tags=['geospatial', 'nuts', 'gisco'],
)
```

### Task Flow

```
start
  ↓
download_nuts0
  ↓
download_nuts1
  ↓
download_nuts2
  ↓
validate_geometries
  ↓
transform_to_epsg3035
  ↓
calculate_areas
  ↓
load_to_dim_region
  ↓
validate_spatial_data
  ↓
end
```

### Key Tasks

#### download_nuts0/1/2

**Purpose**: Download NUTS boundaries from GISCO

**Logic**:
```python
def download_nuts_boundaries(level, **context):
    import requests
    from pathlib import Path
    
    base_url = 'https://gisco-services.ec.europa.eu/distribution/v2/nuts/geojson/'
    filename = f'NUTS_RG_01M_2021_4326_LEVL_{level}.geojson'
    url = base_url + filename
    
    output_dir = Path('/tmp/nuts_download')
    output_dir.mkdir(exist_ok=True)
    output_file = output_dir / filename
    
    response = requests.get(url, stream=True, timeout=300)
    response.raise_for_status()
    
    with open(output_file, 'wb') as f:
        for chunk in response.iter_content(chunk_size=8192):
            f.write(chunk)
    
    return str(output_file)
```

---

#### transform_to_epsg3035

**Purpose**: Transform geometries to EPSG:3035 for analysis

**Logic**:
- Load GeoJSON with GeoPandas
- Transform from EPSG:4326 to EPSG:3035
- Store both CRS versions in database

---

#### load_to_dim_region

**Purpose**: Load NUTS boundaries to `dim_region` table

**Logic**:
- Use PostGIS to insert geometries
- Handle updates (ON CONFLICT DO UPDATE)
- Preserve existing data if geometry unchanged

---

## DAG 4: Data Quality Check

### Purpose

Run automated data quality validation checks.

### Schedule

- **Frequency**: Daily (02:00 UTC)
- **Timeout**: 1 hour

### Task Flow

```
start
  ↓
check_completeness
  ↓
check_validity
  ↓
check_consistency
  ↓
check_anomalies
  ↓
generate_quality_report
  ↓
end
```

### Quality Checks

1. **Completeness**:
   - Check for missing countries/years
   - Check for NULL values in required fields
   - Threshold: <5% missing values

2. **Validity**:
   - Check value ranges (non-negative, reasonable)
   - Check foreign key integrity
   - Check date ranges

3. **Consistency**:
   - Check country totals match sum of regions
   - Check year-over-year changes are reasonable
   - Check unit consistency

4. **Anomalies**:
   - Detect outliers (statistical)
   - Detect sudden changes
   - Flag for manual review

**Results**: Stored in `meta_data_quality` table

---

## DAG 5: ML Dataset Generation

### Purpose

Generate ML-ready datasets from harmonized data.

### Schedule

- **Frequency**: Weekly (after harmonization)
- **Trigger**: External trigger
- **Timeout**: 2 hours

### Task Flow

```
start
  ↓
extract_features
  ↓
join_with_regions
  ↓
calculate_aggregates
  ↓
create_clustering_dataset
  ↓
create_forecasting_dataset
  ↓
validate_ml_datasets
  ↓
end
```

---

## Common Patterns

### Database Connection

```python
from airflow.providers.postgres.hooks.postgres import PostgresHook

def get_db_connection():
    return PostgresHook(postgres_conn_id='lianel_energy_db')
```

**Connection ID**: `lianel_energy_db`  
**Connection Type**: Postgres  
**Host**: `172.18.0.1` (host PostgreSQL)  
**Database**: `lianel_energy`  
**Schema**: `public`

---

### Error Logging

```python
def log_ingestion_error(source_system, table_name, error_message, **context):
    db_hook = get_db_connection()
    
    sql = """
        INSERT INTO meta_ingestion_log 
        (source_system, table_name, status, error_message, ingestion_timestamp)
        VALUES (%s, %s, 'failed', %s, NOW())
    """
    
    db_hook.run(sql, parameters=(source_system, table_name, error_message))
```

---

### Idempotency Check

```python
def check_data_exists(country_code, year, table_code, **context):
    db_hook = get_db_connection()
    
    sql = """
        SELECT COUNT(*) 
        FROM meta_ingestion_log
        WHERE source_system = 'eurostat'
          AND table_name = %s
          AND status = 'success'
          AND ingestion_timestamp > NOW() - INTERVAL '7 days'
    """
    
    count = db_hook.get_first(sql, parameters=(table_code,))[0]
    return count > 0
```

---

## Task Dependencies

### Eurostat Ingestion Dependencies

```
start → validate_connections → check_last_ingestion
  ↓
  └→ [fetch_table_data] (parallel for each table/country/year)
      ↓
      validate_response → transform_to_staging → load_to_staging
  ↓
log_ingestion_summary → end
```

### Harmonization Dependencies

```
start → validate_staging_data
  ↓
convert_units_to_gwh → normalize_country_codes
  ↓
map_product_codes → map_flow_codes
  ↓
calculate_derived_metrics → load_to_fact_table
  ↓
validate_harmonized_data → end
```

---

## Monitoring and Alerting

### Metrics to Track

1. **DAG Execution**:
   - Success rate
   - Execution time
   - Task failures

2. **Data Quality**:
   - Records ingested
   - Quality check results
   - Error rates

3. **Performance**:
   - API response times
   - Database query times
   - Task durations

### Alerts

1. **DAG Failures**: Alert if DAG fails 3 times in a row
2. **Data Quality**: Alert if quality checks fail
3. **Performance**: Alert if execution time > 2x normal
4. **API Issues**: Alert if API errors > 10%

---

## Implementation Checklist

### Phase 1: Setup
- [ ] Create Airflow connection: `lianel_energy_db`
- [ ] Install required Python packages in Airflow image
- [ ] Create DAG folder structure
- [ ] Set up logging configuration

### Phase 2: DAG Development
- [ ] Implement `eurostat_ingestion` DAG
- [ ] Implement `eurostat_harmonization` DAG
- [ ] Implement `nuts_boundary_update` DAG
- [ ] Implement `data_quality_check` DAG
- [ ] Test each DAG individually

### Phase 3: Integration
- [ ] Test end-to-end pipeline
- [ ] Set up monitoring dashboards
- [ ] Configure alerts
- [ ] Document DAG usage

---

## References

- **Airflow Documentation**: https://airflow.apache.org/docs/
- **PostgreSQL Hook**: https://airflow.apache.org/docs/apache-airflow-providers-postgres/
- **Eurostat API**: `02-data-inventory/01-eurostat-inventory.md`
- **NUTS GISCO**: `02-data-inventory/04-nuts-geospatial-inventory.md`
- **Database Schema**: `04-data-model/05-schema-ddl.sql`

---

**Status**: ✅ Airflow DAG design complete. Ready for implementation.

