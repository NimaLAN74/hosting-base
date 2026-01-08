"""
Eurostat Data Ingestion DAG

This DAG ingests annual energy data from Eurostat API for all EU27 countries.
It processes priority tables: nrg_bal_s, nrg_cb_e, nrg_ind_eff, nrg_ind_ren.

Schedule: Weekly (every Sunday at 02:00 UTC)
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

# EU27 country codes
EU27_COUNTRIES = [
    'AT', 'BE', 'BG', 'CY', 'CZ', 'DE', 'DK', 'EE', 'EL', 'ES', 'FI', 'FR',
    'HR', 'HU', 'IE', 'IT', 'LT', 'LU', 'LV', 'MT', 'NL', 'PL', 'PT', 'RO',
    'SE', 'SI', 'SK'
]

# Priority tables to ingest
# Expanded to load more years (2015-2024) for better historical coverage
PRIORITY_TABLES = {
    'nrg_bal_s': {'years': list(range(2015, 2025))},  # Energy balance - supply
    'nrg_cb_e': {'years': list(range(2015, 2025))},   # Energy balances - electricity
    'nrg_ind_eff': {'years': list(range(2015, 2025))}, # Energy efficiency indicators
    'nrg_ind_ren': {'years': list(range(2015, 2025))}, # Renewable energy indicators
}

BASE_URL = 'https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data'


def decode_eurostat_dimension(dimensions: Dict, dim_name: str, position: int, dim_order: List[str], dim_sizes: List[int]) -> Optional[str]:
    """
    Decode Eurostat dimension index to actual value.
    
    Eurostat uses position-based indexing where each value key represents
    a position in a multi-dimensional array.
    
    Args:
        dimensions: The dimension dictionary from API response
        dim_name: Name of the dimension to decode (e.g., 'siec', 'nrg_bal')
        position: The position index from the value key
        dim_order: List of dimension names in order (from 'id' field)
        dim_sizes: List of dimension sizes (from 'size' field)
    
    Returns:
        The decoded dimension value or None
    """
    if dim_name not in dimensions:
        return None
    
    # Find the index of this dimension in the order
    try:
        dim_index = dim_order.index(dim_name)
    except ValueError:
        return None
    
    # Calculate the position within this dimension
    # We need to extract the position for this specific dimension
    # by dividing by the product of sizes of dimensions after it
    divisor = 1
    for i in range(dim_index + 1, len(dim_sizes)):
        divisor *= dim_sizes[i]
    
    dim_position = (position // divisor) % dim_sizes[dim_index]
    
    # Get the category index mapping
    dim_data = dimensions[dim_name]
    if 'category' in dim_data and 'index' in dim_data['category']:
        index_map = dim_data['category']['index']
        # Find the key that maps to this position
        for key, idx in index_map.items():
            if idx == dim_position:
                return key
    
    return None


def validate_connections(**context):
    """Validate database and API connections"""
    try:
        # Test database connection
        db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
        conn = db_hook.get_conn()
        cursor = conn.cursor()
        cursor.execute("SELECT 1")
        cursor.close()
        conn.close()
        print("✅ Database connection validated")
        
        # Test API connection
        response = requests.get(f'{BASE_URL}/nrg_bal_s?format=JSON&geo=SE&time=2022', timeout=30)
        if response.status_code == 200:
            print("✅ Eurostat API connection validated")
        else:
            raise AirflowException(f"API returned status {response.status_code}")
            
    except Exception as e:
        raise AirflowException(f"Connection validation failed: {e}")


def check_last_ingestion(**context):
    """Check last successful ingestion and determine what needs to be ingested"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    # Get last successful ingestion for each table
    ingestion_plan = {}
    for table_code in PRIORITY_TABLES.keys():
        sql = """
            SELECT MAX(ingestion_timestamp) as last_ingestion
            FROM meta_ingestion_log
            WHERE source_system = 'eurostat'
              AND table_name = %s
              AND status = 'success'
        """
        result = db_hook.get_first(sql, parameters=(table_code,))
        last_ingestion = result[0] if result and result[0] else None
        
        # Determine years to ingest (for now, use configured years)
        years = PRIORITY_TABLES[table_code]['years']
        ingestion_plan[table_code] = {
            'years': years,
            'last_ingestion': str(last_ingestion) if last_ingestion else None
        }
    
    # Store in XCom
    context['ti'].xcom_push(key='ingestion_plan', value=ingestion_plan)
    print(f"Ingestion plan: {json.dumps(ingestion_plan, indent=2, default=str)}")
    return ingestion_plan


def fetch_eurostat_data(table_code: str, country_code: str, year: int, **context):
    """Fetch data from Eurostat API for a specific table, country, and year"""
    params = {
        'format': 'JSON',
        'geo': country_code,
        'time': str(year)
    }
    
    try:
        response = requests.get(
            f'{BASE_URL}/{table_code}',
            params=params,
            timeout=60
        )
        response.raise_for_status()
        
        data = response.json()
        
        # Validate response structure
        if 'value' not in data or not data['value']:
            print(f"⚠️  No data for {country_code} {year} in {table_code}")
            return None
        
        # Rate limiting - be respectful
        time.sleep(0.5)
        
        return {
            'data': data,
            'record_count': len(data['value']),
            'table_code': table_code,
            'country_code': country_code,
            'year': year
        }
        
    except requests.RequestException as e:
        raise AirflowException(f"API request failed for {table_code} {country_code} {year}: {e}")


def transform_eurostat_data(table_code: str, country_code: str, year: int, **context):
    """Transform Eurostat JSON response to flat records"""
    ti = context['ti']
    
    # Get data from previous task (with TaskGroup prefix)
    task_group_id = f'ingest_{table_code}'
    fetch_task_id = f'{task_group_id}.fetch_{table_code}_{country_code}_{year}'
    api_data = ti.xcom_pull(
        task_ids=fetch_task_id,
        key='return_value'
    )
    
    if not api_data:
        return []
    
    data = api_data['data']
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
        
        # Decode dimensions
        product_code = None
        flow_code = None
        unit_code = None
        
        # Decode based on table structure
        if 'siec' in dim_order:
            product_code = decode_eurostat_dimension(dimensions, 'siec', position, dim_order, dim_sizes)
        
        if 'nrg_bal' in dim_order:
            flow_code = decode_eurostat_dimension(dimensions, 'nrg_bal', position, dim_order, dim_sizes)
        
        if 'unit' in dim_order:
            unit_code = decode_eurostat_dimension(dimensions, 'unit', position, dim_order, dim_sizes)
        
        # Only create record if we have essential dimensions
        if product_code or flow_code:
            record = {
                'country_code': country_code,
                'year': year,
                'product_code': product_code,
                'flow_code': flow_code,
                'value_raw': float(value) if value else None,
                'unit': unit_code,  # Map to 'unit' column
                'source_table': table_code,
                'source_system': 'eurostat',
            }
            records.append(record)
    
    print(f"✅ Transformed {len(records)} records for {table_code} {country_code} {year}")
    return records


def load_to_staging(table_code: str, country_code: str, year: int, **context):
    """Load transformed data to staging table (fact_energy_annual)"""
    ti = context['ti']
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    # Get transformed records (with TaskGroup prefix)
    task_group_id = f'ingest_{table_code}'
    transform_task_id = f'{task_group_id}.transform_{table_code}_{country_code}_{year}'
    records = ti.xcom_pull(
        task_ids=transform_task_id,
        key='return_value'
    )
    
    if not records:
        print(f"No records to load for {table_code} {country_code} {year}")
        return 0
    
    # Bulk insert to fact_energy_annual
    # Note: Unit conversion to GWh will be done in harmonization DAG
    # For now, store raw value in value_gwh (will be converted later)
    insert_sql = """
        INSERT INTO fact_energy_annual 
        (country_code, year, product_code, flow_code, value_gwh, unit, source_table, source_system, ingestion_timestamp)
        VALUES (%s, %s, %s, %s, %s, %s, %s, %s, NOW())
        ON CONFLICT (country_code, year, product_code, flow_code, source_table) 
        DO UPDATE SET
            value_gwh = EXCLUDED.value_gwh,
            unit = COALESCE(EXCLUDED.unit, fact_energy_annual.unit),
            ingestion_timestamp = EXCLUDED.ingestion_timestamp
    """
    
    inserted = 0
    for record in records:
        try:
            db_hook.run(insert_sql, parameters=(
                record['country_code'],
                record['year'],
                record['product_code'],
                record['flow_code'],
                record['value_raw'],  # Will be converted to GWh in harmonization
                record['unit'],
                record['source_table'],
                record['source_system']
            ))
            inserted += 1
        except Exception as e:
            print(f"⚠️  Error inserting record: {e}")
            continue
    
    print(f"✅ Loaded {inserted}/{len(records)} records for {table_code} {country_code} {year}")
    return inserted


def log_ingestion_summary(**context):
    """Log ingestion results to metadata table"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    # Aggregate results from all tasks
    # For now, we'll query the database to get actual counts
    sql = """
        SELECT 
            source_table,
            COUNT(*) as records_inserted
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
          AND ingestion_timestamp >= CURRENT_DATE
        GROUP BY source_table
    """
    
    results = db_hook.get_records(sql)
    
    total_inserted = 0
    for table_code, count in results:
        total_inserted += count
        summary = {
            'source_system': 'eurostat',
            'table_name': table_code,
            'status': 'success',
            'records_inserted': count,
            'records_updated': 0,
            'records_failed': 0,
        }
        
        log_sql = """
            INSERT INTO meta_ingestion_log 
            (source_system, table_name, status, records_inserted, records_updated, 
             records_failed, ingestion_timestamp)
            VALUES (%s, %s, %s, %s, %s, %s, NOW())
        """
        
        db_hook.run(log_sql, parameters=(
            summary['source_system'],
            summary['table_name'],
            summary['status'],
            summary['records_inserted'],
            summary['records_updated'],
            summary['records_failed']
        ))
    
    print(f"✅ Ingestion complete: {total_inserted} total records inserted")
    return {'total_inserted': total_inserted}


# DAG Tasks
start_task = PythonOperator(
    task_id='start',
    python_callable=lambda: print("🚀 Starting Eurostat ingestion"),
    dag=dag,
)

validate_connections_task = PythonOperator(
    task_id='validate_connections',
    python_callable=validate_connections,
    dag=dag,
)

check_last_ingestion_task = PythonOperator(
    task_id='check_last_ingestion',
    python_callable=check_last_ingestion,
    dag=dag,
)

# Dynamic task generation for each table/country/year combination
ingestion_tasks = []
table_groups = []

for table_code, table_config in PRIORITY_TABLES.items():
    with TaskGroup(f'ingest_{table_code}', dag=dag) as table_group:
        table_load_tasks = []
        
        for country_code in EU27_COUNTRIES:
            for year in table_config['years']:
                task_id_prefix = f'{table_code}_{country_code}_{year}'
                
                fetch_task = PythonOperator(
                    task_id=f'fetch_{task_id_prefix}',
                    python_callable=fetch_eurostat_data,
                    op_kwargs={
                        'table_code': table_code,
                        'country_code': country_code,
                        'year': year
                    },
                    dag=dag,
                )
                
                transform_task = PythonOperator(
                    task_id=f'transform_{task_id_prefix}',
                    python_callable=transform_eurostat_data,
                    op_kwargs={
                        'table_code': table_code,
                        'country_code': country_code,
                        'year': year
                    },
                    dag=dag,
                )
                
                load_task = PythonOperator(
                    task_id=f'load_{task_id_prefix}',
                    python_callable=load_to_staging,
                    op_kwargs={
                        'table_code': table_code,
                        'country_code': country_code,
                        'year': year
                    },
                    dag=dag,
                )
                
                # Task dependencies within group
                fetch_task >> transform_task >> load_task
                table_load_tasks.append(load_task)
        
        table_groups.append(table_group)
        ingestion_tasks.extend(table_load_tasks)

log_summary_task = PythonOperator(
    task_id='log_ingestion_summary',
    python_callable=log_ingestion_summary,
    dag=dag,
)

end_task = PythonOperator(
    task_id='end',
    python_callable=lambda: print("✅ Eurostat ingestion complete"),
    dag=dag,
)

# DAG dependencies
start_task >> validate_connections_task >> check_last_ingestion_task

# Connect all table groups to check_last_ingestion
for table_group in table_groups:
    check_last_ingestion_task >> table_group

# Connect all ingestion tasks to summary (using list expansion)
if ingestion_tasks:
    ingestion_tasks[0] >> log_summary_task
    for i in range(1, len(ingestion_tasks)):
        ingestion_tasks[i] >> log_summary_task

log_summary_task >> end_task
