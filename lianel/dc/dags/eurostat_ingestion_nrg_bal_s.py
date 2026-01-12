"""
Eurostat Data Ingestion DAG - Per Table Template

This is a template for creating per-table ingestion DAGs.
Each table gets its own DAG to reduce task count and enable better failure recovery.

Features:
- Checkpoint/resume logic: Skips already-ingested data
- Batch processing: Processes all years for a country in one task
- Independent execution: Each table DAG can run independently
- Failure recovery: Can restart from last successful checkpoint

Usage: Copy this template and set TABLE_CODE and TABLE_CONFIG
"""
from datetime import datetime, timedelta
from airflow import DAG
from airflow.operators.python import PythonOperator
from airflow.providers.postgres.hooks.postgres import PostgresHook
from airflow.utils.task_group import TaskGroup
from airflow.exceptions import AirflowException
import requests
import json
import time
from typing import Dict, List, Any, Optional

# ============================================================================
# CONFIGURATION - Set these for each table DAG
# ============================================================================
TABLE_CODE = 'nrg_bal_s'  # Change this for each table
TABLE_YEARS = list(range(2015, 2025))  # Years to ingest
TABLE_DESCRIPTION = 'Energy balance - supply'  # Description for this table

# ============================================================================

default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'start_date': datetime(2026, 1, 1),
    'email_on_failure': False,
    'email_on_retry': False,
    'retries': 2,
    'retry_delay': timedelta(minutes=5),
    'execution_timeout': timedelta(hours=2),  # Reduced per table
}

dag = DAG(
    f'eurostat_ingestion_{TABLE_CODE}',
    default_args=default_args,
    description=f'Ingest Eurostat {TABLE_CODE} data: {TABLE_DESCRIPTION}',
    schedule='0 2 * * 0',  # Every Sunday at 02:00 UTC
    catchup=False,
    tags=['data-ingestion', 'eurostat', 'energy', TABLE_CODE],
    max_active_runs=1,
)

# EU27 country codes
EU27_COUNTRIES = [
    'AT', 'BE', 'BG', 'CY', 'CZ', 'DE', 'DK', 'EE', 'EL', 'ES', 'FI', 'FR',
    'HR', 'HU', 'IE', 'IT', 'LT', 'LU', 'LV', 'MT', 'NL', 'PL', 'PT', 'RO',
    'SE', 'SI', 'SK'
]

BASE_URL = 'https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data'


def decode_eurostat_dimension(dimensions: Dict, dim_name: str, position: int, dim_order: List[str], dim_sizes: List[int]) -> Optional[str]:
    """Decode Eurostat dimension index to actual value."""
    if dim_name not in dimensions:
        return None
    
    try:
        dim_index = dim_order.index(dim_name)
    except ValueError:
        return None
    
    divisor = 1
    for i in range(dim_index + 1, len(dim_sizes)):
        divisor *= dim_sizes[i]
    
    dim_position = (position // divisor) % dim_sizes[dim_index]
    
    dim_data = dimensions[dim_name]
    if 'category' in dim_data and 'index' in dim_data['category']:
        index_map = dim_data['category']['index']
        for key, idx in index_map.items():
            if idx == dim_position:
                return key
    
    return None


def check_ingestion_checkpoint(**context):
    """Check what data has already been ingested and create ingestion plan"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    # Get already-ingested country/year combinations with harmonization version check
    # Only skip if data was harmonized recently (within last 7 days) to allow re-ingestion of missing products
    sql = """
        SELECT DISTINCT country_code, year
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
          AND source_table = %s
          AND harmonisation_version IS NOT NULL
          AND ingestion_timestamp >= CURRENT_DATE - INTERVAL '7 days'
    """
    ingested = db_hook.get_records(sql, parameters=(TABLE_CODE,))
    ingested_set = {(row[0], row[1]) for row in ingested} if ingested else set()
    
    # Create ingestion plan: only process what's not already ingested and harmonized
    ingestion_plan = []
    for country_code in EU27_COUNTRIES:
        for year in TABLE_YEARS:
            if (country_code, year) not in ingested_set:
                ingestion_plan.append({
                    'country_code': country_code,
                    'year': year,
                    'skip': False
                })
            else:
                ingestion_plan.append({
                    'country_code': country_code,
                    'year': year,
                    'skip': True
                })
    
    # Group by country for batch processing
    country_batches = {}
    for item in ingestion_plan:
        country = item['country_code']
        if country not in country_batches:
            country_batches[country] = []
        country_batches[country].append(item)
    
    # Store in XCom
    context['ti'].xcom_push(key='ingestion_plan', value=ingestion_plan)
    context['ti'].xcom_push(key='country_batches', value=country_batches)
    
    total_to_process = sum(1 for item in ingestion_plan if not item['skip'])
    total_to_skip = sum(1 for item in ingestion_plan if item['skip'])
    
    print(f"📋 Ingestion plan for {TABLE_CODE}:")
    print(f"   Total combinations: {len(ingestion_plan)}")
    print(f"   To process: {total_to_process}")
    print(f"   To skip (already ingested): {total_to_skip}")
    
    return {
        'total': len(ingestion_plan),
        'to_process': total_to_process,
        'to_skip': total_to_skip,
        'country_batches': country_batches
    }


def fetch_and_load_country_batch(country_code: str, **context):
    """Fetch and load all years for a country in one task (batch processing)"""
    ti = context['ti']
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    # Get ingestion plan for this country
    country_batches = ti.xcom_pull(task_ids='check_ingestion_checkpoint', key='country_batches')
    if not country_batches or country_code not in country_batches:
        print(f"⚠️  No ingestion plan for {country_code}")
        return 0
    
    country_plan = country_batches[country_code]
    total_inserted = 0
    
    for item in country_plan:
        if item['skip']:
            print(f"⏭️  Skipping {country_code} {item['year']} (already ingested)")
            continue
        
        year = item['year']
        
        # Fetch data
        try:
            params = {
                'format': 'JSON',
                'geo': country_code,
                'time': str(year)
            }
            
            response = requests.get(
                f'{BASE_URL}/{TABLE_CODE}',
                params=params,
                timeout=60
            )
            response.raise_for_status()
            data = response.json()
            
            if 'value' not in data or not data['value']:
                print(f"⚠️  No data for {country_code} {year}")
                continue
            
            # Transform data
            dimensions = data.get('dimension', {})
            values = data.get('value', {})
            dim_order = data.get('id', [])
            dim_sizes = data.get('size', [])
            
            records = []
            for key_str, value in values.items():
                try:
                    position = int(key_str)
                except ValueError:
                    continue
                
                product_code = None
                flow_code = None
                unit_code = None
                
                if 'siec' in dim_order:
                    product_code = decode_eurostat_dimension(dimensions, 'siec', position, dim_order, dim_sizes)
                if 'nrg_bal' in dim_order:
                    flow_code = decode_eurostat_dimension(dimensions, 'nrg_bal', position, dim_order, dim_sizes)
                if 'unit' in dim_order:
                    unit_code = decode_eurostat_dimension(dimensions, 'unit', position, dim_order, dim_sizes)
                
                # Only include records with valid product_code, flow_code, and non-null value
                if product_code and flow_code and value is not None:
                    try:
                        value_float = float(value)
                        if value_float > 0:  # Only include positive values
                            records.append({
                                'country_code': country_code,
                                'year': year,
                                'product_code': product_code,
                                'flow_code': flow_code,
                                'value_raw': value_float,
                                'unit': unit_code,
                                'source_table': TABLE_CODE,
                                'source_system': 'eurostat',
                            })
                    except (ValueError, TypeError):
                        continue  # Skip invalid values
            
            # Bulk insert
            if records:
                insert_sql = """
                    INSERT INTO fact_energy_annual 
                    (country_code, year, product_code, flow_code, value_gwh, unit, source_table, source_system, ingestion_timestamp)
                    VALUES (%s, %s, %s, %s, %s, %s, %s, %s, NOW())
                    ON CONFLICT (country_code, year, product_code, flow_code, source_table) 
                    DO UPDATE SET
                        value_gwh = EXCLUDED.value_gwh,
                        unit = COALESCE(EXCLUDED.unit, fact_energy_annual.unit),
                        ingestion_timestamp = EXCLUDED.ingestion_timestamp,
                        harmonisation_version = NULL  -- Reset harmonization version to allow re-harmonization
                """
                
                for record in records:
                    try:
                        # Check if product_code exists in dim_energy_product before inserting
                        check_product_sql = "SELECT 1 FROM dim_energy_product WHERE product_code = %s LIMIT 1"
                        product_exists = db_hook.get_first(check_product_sql, parameters=(record['product_code'],))
                        
                        if not product_exists:
                            # Skip records with unknown product codes
                            continue
                        
                        # Check if flow_code exists in dim_energy_flow before inserting
                        check_flow_sql = "SELECT 1 FROM dim_energy_flow WHERE flow_code = %s LIMIT 1"
                        flow_exists = db_hook.get_first(check_flow_sql, parameters=(record['flow_code'],))
                        
                        if not flow_exists:
                            # Skip records with unknown flow codes
                            continue
                        
                        db_hook.run(insert_sql, parameters=(
                            record['country_code'],
                            record['year'],
                            record['product_code'],
                            record['flow_code'],
                            record['value_raw'],
                            record['unit'],
                            record['source_table'],
                            record['source_system']
                        ))
                        total_inserted += 1
                    except Exception as e:
                        print(f"⚠️  Error inserting record: {e}")
                        continue
                
                print(f"✅ Loaded {len(records)} records for {country_code} {year}")
            
            # Rate limiting
            time.sleep(0.5)
            
        except Exception as e:
            print(f"❌ Error processing {country_code} {year}: {e}")
            continue
    
    print(f"✅ Batch complete for {country_code}: {total_inserted} total records inserted")
    return total_inserted


def log_table_summary(**context):
    """Log ingestion summary for this table"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    sql = """
        SELECT COUNT(*) as records_inserted
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
          AND source_table = %s
          AND ingestion_timestamp >= CURRENT_DATE
    """
    result = db_hook.get_first(sql, parameters=(TABLE_CODE,))
    count = result[0] if result else 0
    
    log_sql = """
        INSERT INTO meta_ingestion_log 
        (source_system, table_name, status, records_inserted, records_updated, 
         records_failed, ingestion_timestamp)
        VALUES (%s, %s, %s, %s, %s, %s, NOW())
    """
    
    db_hook.run(log_sql, parameters=(
        'eurostat',
        TABLE_CODE,
        'success',
        count,
        0,
        0
    ))
    
    print(f"✅ {TABLE_CODE} ingestion logged: {count} records")
    return {'table': TABLE_CODE, 'records': count}


# DAG Tasks
start_task = PythonOperator(
    task_id='start',
    python_callable=lambda: print(f"🚀 Starting {TABLE_CODE} ingestion"),
    dag=dag,
)

checkpoint_task = PythonOperator(
    task_id='check_ingestion_checkpoint',
    python_callable=check_ingestion_checkpoint,
    dag=dag,
)

# Create one task per country (batch processing)
country_tasks = []
for country_code in EU27_COUNTRIES:
    country_task = PythonOperator(
        task_id=f'fetch_and_load_{country_code}',
        python_callable=fetch_and_load_country_batch,
        op_kwargs={'country_code': country_code},
        dag=dag,
    )
    country_tasks.append(country_task)

log_summary_task = PythonOperator(
    task_id='log_table_summary',
    python_callable=log_table_summary,
    dag=dag,
)

end_task = PythonOperator(
    task_id='end',
    python_callable=lambda: print(f"✅ {TABLE_CODE} ingestion complete"),
    dag=dag,
)

# DAG dependencies
start_task >> checkpoint_task
checkpoint_task >> country_tasks
country_tasks >> log_summary_task
log_summary_task >> end_task
