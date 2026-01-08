"""
Eurostat Data Ingestion DAG - TEST VERSION

This is a test version that processes only one country/year to verify the pipeline works.
Once validated, use the full eurostat_ingestion_dag.py
"""
from datetime import datetime, timedelta
from airflow import DAG
from airflow.operators.python import PythonOperator
from airflow.providers.postgres.hooks.postgres import PostgresHook
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
    'retries': 2,
    'retry_delay': timedelta(minutes=5),
    'execution_timeout': timedelta(hours=2),
}

dag = DAG(
    'eurostat_ingestion_test',
    default_args=default_args,
    description='TEST: Ingest Eurostat data for one country/year',
    schedule=None,  # Manual trigger only
    catchup=False,
    tags=['test', 'data-ingestion', 'eurostat'],
    max_active_runs=1,
)

BASE_URL = 'https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data'

# TEST: Only one country, one year, one table
TEST_COUNTRY = 'SE'
TEST_YEAR = 2022
TEST_TABLE = 'nrg_bal_s'


def decode_eurostat_dimension(dimensions: Dict, dim_name: str, position: int, dim_order: List[str], dim_sizes: List[int]) -> Optional[str]:
    """Decode Eurostat dimension index to actual value"""
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


def test_fetch(**context):
    """Test fetch from Eurostat API"""
    params = {
        'format': 'JSON',
        'geo': TEST_COUNTRY,
        'time': str(TEST_YEAR)
    }
    
    try:
        response = requests.get(
            f'{BASE_URL}/{TEST_TABLE}',
            params=params,
            timeout=60
        )
        response.raise_for_status()
        data = response.json()
        
        if 'value' not in data or not data['value']:
            raise AirflowException(f"No data returned for {TEST_COUNTRY} {TEST_YEAR}")
        
        print(f"✅ Fetched {len(data['value'])} values from API")
        context['ti'].xcom_push(key='api_data', value=data)
        return data
        
    except requests.RequestException as e:
        raise AirflowException(f"API request failed: {e}")


def test_transform(**context):
    """Test transformation"""
    ti = context['ti']
    data = ti.xcom_pull(task_ids='test_fetch', key='api_data')
    
    if not data:
        raise AirflowException("No data from fetch task")
    
    dimensions = data.get('dimension', {})
    values = data.get('value', {})
    dim_order = data.get('id', [])
    dim_sizes = data.get('size', [])
    
    records = []
    sample_count = 0
    
    for key_str, value in list(values.items())[:10]:  # Test with first 10 values
        try:
            position = int(key_str)
        except ValueError:
            continue
        
        product_code = decode_eurostat_dimension(dimensions, 'siec', position, dim_order, dim_sizes) if 'siec' in dim_order else None
        flow_code = decode_eurostat_dimension(dimensions, 'nrg_bal', position, dim_order, dim_sizes) if 'nrg_bal' in dim_order else None
        unit_code = decode_eurostat_dimension(dimensions, 'unit', position, dim_order, dim_sizes) if 'unit' in dim_order else None
        
        if product_code or flow_code:
            record = {
                'country_code': TEST_COUNTRY,
                'year': TEST_YEAR,
                'product_code': product_code,
                'flow_code': flow_code,
                'value_raw': float(value) if value else None,
                'unit': unit_code,
                'source_table': TEST_TABLE,
                'source_system': 'eurostat',
            }
            records.append(record)
            sample_count += 1
    
    print(f"✅ Transformed {len(records)} sample records")
    print(f"Sample record: {json.dumps(records[0] if records else {}, indent=2)}")
    
    ti.xcom_push(key='transformed_records', value=records)
    return records


def test_load(**context):
    """Test loading to database"""
    ti = context['ti']
    records = ti.xcom_pull(task_ids='test_transform', key='transformed_records')
    
    if not records:
        raise AirflowException("No records to load")
    
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
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
                record['value_raw'],
                record['unit'],
                record['source_table'],
                record['source_system']
            ))
            inserted += 1
        except Exception as e:
            print(f"⚠️  Error inserting record: {e}")
            continue
    
    print(f"✅ Loaded {inserted}/{len(records)} records to database")
    
    # Verify data
    verify_sql = """
        SELECT COUNT(*) as count, 
               COUNT(DISTINCT product_code) as products,
               COUNT(DISTINCT flow_code) as flows
        FROM fact_energy_annual
        WHERE country_code = %s AND year = %s AND source_table = %s
    """
    result = db_hook.get_first(verify_sql, parameters=(TEST_COUNTRY, TEST_YEAR, TEST_TABLE))
    print(f"✅ Verification: {result[0]} records, {result[1]} products, {result[2]} flows")
    
    return inserted


# Tasks
fetch_task = PythonOperator(
    task_id='test_fetch',
    python_callable=test_fetch,
    dag=dag,
)

transform_task = PythonOperator(
    task_id='test_transform',
    python_callable=test_transform,
    dag=dag,
)

load_task = PythonOperator(
    task_id='test_load',
    python_callable=test_load,
    dag=dag,
)

fetch_task >> transform_task >> load_task

