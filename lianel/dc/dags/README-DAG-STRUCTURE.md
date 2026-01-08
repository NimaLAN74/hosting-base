# Eurostat Ingestion DAG Structure

## Overview

The Eurostat ingestion pipeline has been restructured to break down the heavy monolithic DAG into smaller, manageable per-table DAGs with checkpoint/resume capabilities.

## Problem Solved

**Before:**
- Single DAG with 3,240 tasks (4 tables × 27 countries × 10 years × 3 tasks)
- Too heavy for Airflow scheduler
- Failure required rerunning all tasks
- No checkpoint/resume capability

**After:**
- 4 per-table DAGs with ~27 tasks each (one per country)
- Total: ~108 tasks instead of 3,240
- Checkpoint logic: skips already-ingested data
- Batch processing: all years for a country in one task
- Failure recovery: can restart from last successful checkpoint

## DAG Structure

### 1. Per-Table Ingestion DAGs

Each table has its own DAG:
- `eurostat_ingestion_nrg_bal_s` - Energy balance - supply
- `eurostat_ingestion_nrg_cb_e` - Energy balances - electricity
- `eurostat_ingestion_nrg_ind_eff` - Energy efficiency indicators
- `eurostat_ingestion_nrg_ind_ren` - Renewable energy indicators

**Structure:**
```
start
  ↓
check_ingestion_checkpoint (determines what to ingest)
  ↓
fetch_and_load_{country} (one task per country, batches all years)
  ↓
log_table_summary
  ↓
end
```

**Features:**
- **Checkpoint Logic**: Checks what's already ingested in the last 7 days and skips it
- **Batch Processing**: Each country task processes all years (2015-2024) in one go
- **Failure Recovery**: Can rerun individual country tasks without reprocessing others
- **Independent Execution**: Each table DAG can run independently

### 2. Coordinator DAG

`eurostat_ingestion_coordinator` - Coordinates all table DAGs

**Structure:**
```
start
  ↓
wait_for_{table_dag} (ExternalTaskSensor for each table)
  ↓
check_all_tables_complete (verifies all succeeded)
  ↓
trigger_harmonization (triggers harmonization DAG)
  ↓
end
```

**Features:**
- Uses `ExternalTaskSensor` to wait for all table DAGs to complete
- Verifies all tables ingested successfully
- Automatically triggers harmonization DAG after completion

### 3. Harmonization DAG

`eurostat_harmonization` - Transforms and harmonizes ingested data

**Triggered by:** Coordinator DAG (after all ingestion completes)

**Features:**
- Unit conversion to GWh
- Code mapping and normalization
- Data quality validation

## How It Works

### Normal Flow

1. **Sunday 02:00 UTC**: All table DAGs start simultaneously
2. Each table DAG:
   - Checks checkpoint (what's already ingested)
   - Processes only missing country/year combinations
   - Logs summary
3. **Coordinator DAG**:
   - Waits for all table DAGs to complete
   - Verifies success
   - Triggers harmonization
4. **Harmonization DAG**:
   - Transforms and harmonizes data
   - Validates quality

### Failure Recovery

If a table DAG fails:

1. **Rerun the failed table DAG**: It will skip already-ingested data
2. **Checkpoint logic**: Only processes missing combinations
3. **No need to rerun other tables**: Each table is independent

If a country task fails:

1. **Rerun just that country task**: Processes all years for that country
2. **Other countries unaffected**: Already processed data remains

### Manual Execution

You can manually trigger any DAG:

```bash
# Trigger a specific table DAG
airflow dags trigger eurostat_ingestion_nrg_bal_s

# Trigger coordinator (will wait for all tables)
airflow dags trigger eurostat_ingestion_coordinator

# Trigger harmonization manually
airflow dags trigger eurostat_harmonization
```

## Configuration

### Table Configuration

Each table DAG has these configurable variables:

```python
TABLE_CODE = 'nrg_bal_s'  # Table code
TABLE_YEARS = list(range(2015, 2025))  # Years to ingest
TABLE_DESCRIPTION = 'Energy balance - supply'  # Description
```

### Checkpoint Window

The checkpoint logic checks for data ingested in the last 7 days:

```python
WHERE ingestion_timestamp >= CURRENT_DATE - INTERVAL '7 days'
```

To change this, modify the `check_ingestion_checkpoint` function in each table DAG.

## Benefits

1. **Reduced Task Count**: 3,240 → ~108 tasks (97% reduction)
2. **Better Failure Recovery**: Can restart from checkpoint
3. **Independent Execution**: Tables can run in parallel
4. **Easier Debugging**: Smaller DAGs are easier to understand
5. **Resource Efficiency**: Less memory/CPU per DAG run
6. **Scalability**: Can add more tables without increasing task count per DAG

## Monitoring

### Check DAG Status

```sql
-- Check ingestion logs
SELECT 
    table_name,
    status,
    records_inserted,
    ingestion_timestamp
FROM meta_ingestion_log
WHERE source_system = 'eurostat'
  AND ingestion_timestamp >= CURRENT_DATE
ORDER BY table_name;
```

### Check What's Ingested

```sql
-- Check ingested data by table
SELECT 
    source_table,
    COUNT(*) as records,
    COUNT(DISTINCT country_code) as countries,
    COUNT(DISTINCT year) as years,
    MIN(year) as earliest,
    MAX(year) as latest
FROM fact_energy_annual
WHERE source_system = 'eurostat'
GROUP BY source_table;
```

## Troubleshooting

### DAG Not Appearing in Airflow UI

- Check DAG files are in `/dags/` directory
- Verify Python syntax is correct
- Check Airflow logs for import errors

### ExternalTaskSensor Timeout

- Increase `timeout` in coordinator DAG
- Check that table DAGs are actually running
- Verify DAG IDs match exactly

### Checkpoint Not Working

- Verify `meta_ingestion_log` table exists
- Check that ingestion tasks are logging correctly
- Verify date range in checkpoint query

### Harmonization Not Triggered

- Check coordinator DAG completed successfully
- Verify `TriggerDagRunOperator` has correct DAG ID
- Check Airflow logs for trigger errors
