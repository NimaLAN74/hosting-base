"""
ENTSO-E Data Ingestion DAG

This DAG ingests high-frequency electricity data from ENTSO-E Transparency Platform.
Data includes:
- Actual load (demand) per country/bidding zone
- Actual generation by production type

Features:
- Checkpoint/resume logic: Skips already-ingested data
- Batch processing: Processes date ranges in chunks
- Independent execution: Can run independently
- Failure recovery: Can restart from last successful checkpoint
- Timezone handling: Converts local time to UTC
- Subtask breakdown: Separate checkpoint, planning, and ingestion tasks for better debugging
"""

from datetime import datetime, timedelta
from airflow import DAG
from airflow.operators.python import PythonOperator
from airflow.providers.postgres.hooks.postgres import PostgresHook
from airflow.utils.task_group import TaskGroup
from airflow.exceptions import AirflowException
import sys
import os
from typing import Dict, List, Any, Optional

# Add utils directory to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'utils'))
from entsoe_client import ENTSOEClient

default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'start_date': datetime(2026, 1, 1),
    'email_on_failure': False,
    'email_on_retry': False,
    'retries': 2,
    'retry_delay': timedelta(minutes=5),
    'execution_timeout': timedelta(hours=4),
}

dag = DAG(
    'entsoe_ingestion',
    default_args=default_args,
    description='Ingest ENTSO-E electricity data (load and generation)',
    schedule='0 3 * * *',  # Daily at 03:00 UTC (after ENTSO-E data is available)
    catchup=False,
    tags=['data-ingestion', 'entsoe', 'electricity', 'timeseries'],
    max_active_runs=1,
)

# Countries with ENTSO-E data availability (subset of EU27)
ENTSOE_COUNTRIES = [
    'AT', 'BE', 'BG', 'CZ', 'DE', 'DK', 'EE', 'ES', 'FI', 'FR',
    'HR', 'HU', 'IE', 'IT', 'LT', 'LU', 'LV', 'NL', 'PL', 'PT', 'RO',
    'SE', 'SI', 'SK'
]


def check_checkpoint(country_code: str, **context) -> Dict[str, Any]:
    """Check last ingestion date for a country."""
    postgres_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    sql = """
        SELECT MAX(end_timestamp) 
        FROM meta_entsoe_ingestion_log 
        WHERE country_code = %s AND status = 'success'
    """
    
    try:
        result = postgres_hook.get_first(sql, parameters=(country_code,))
        last_date = result[0] if result and result[0] else None
        
        return {
            'country_code': country_code,
            'last_date': last_date.isoformat() if last_date else None,
            'status': 'success'
        }
    except Exception as e:
        print(f"Error checking checkpoint for {country_code}: {e}")
        return {
            'country_code': country_code,
            'last_date': None,
            'status': 'error',
            'error': str(e)
        }


def plan_ingestion(country_code: str, **context) -> Dict[str, Any]:
    """Calculate date ranges to ingest for a country."""
    # Get checkpoint from previous task
    ti = context['ti']
    task_ids = [f'country_{country_code.lower()}.checkpoint_{country_code.lower()}']
    checkpoint_result = ti.xcom_pull(task_ids=task_ids)
    
    last_date_str = None
    if checkpoint_result and isinstance(checkpoint_result, dict):
        last_date_str = checkpoint_result.get('last_date')
    
    if last_date_str:
        last_date = datetime.fromisoformat(last_date_str)
        start_date = (last_date + timedelta(days=1)).strftime('%Y-%m-%d')
    else:
        # Start from 30 days ago (initial load)
        start_date = (datetime.now() - timedelta(days=30)).strftime('%Y-%m-%d')
    
    # End date is yesterday (ENTSO-E data available with 1-day delay)
    end_date = (datetime.now() - timedelta(days=1)).strftime('%Y-%m-%d')
    
    if start_date >= end_date:
        print(f"No new data to ingest for {country_code} (last: {last_date_str})")
        return {
            'country_code': country_code,
            'start_date': None,
            'end_date': None,
            'date_chunks': [],
            'status': 'up_to_date'
        }
    
    # Split into 7-day chunks
    start_dt = datetime.fromisoformat(start_date)
    end_dt = datetime.fromisoformat(end_date)
    
    date_chunks = []
    current_start = start_dt
    
    while current_start < end_dt:
        chunk_end = min(current_start + timedelta(days=7), end_dt)
        date_chunks.append({
            'start': current_start.strftime('%Y-%m-%d'),
            'end': chunk_end.strftime('%Y-%m-%d')
        })
        current_start = chunk_end
    
    return {
        'country_code': country_code,
        'start_date': start_date,
        'end_date': end_date,
        'date_chunks': date_chunks,
        'status': 'ready'
    }


def ingest_date_chunk(country_code: str, start_date: str, end_date: str, **context) -> Dict[str, Any]:
    """
    Ingest ENTSO-E data for a country and date range.
    
    Args:
        country_code: ISO 2-letter country code
        start_date: Start date in format 'YYYY-MM-DD'
        end_date: End date in format 'YYYY-MM-DD'
    """
    postgres_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    # Get API token from Airflow Variable (if set)
    from airflow.models import Variable
    try:
        api_token = Variable.get("ENTSOE_API_TOKEN")
    except KeyError:
        api_token = None
    
    # Initialize ENTSO-E client
    client = ENTSOEClient(api_token=api_token)
    
    print(f"Ingesting ENTSO-E data for {country_code} from {start_date} to {end_date}")
    
    try:
        # Get load data
        load_data = client.get_load_data(country_code, start_date, end_date)
        print(f"Retrieved {len(load_data)} load records")
        
        # Get generation data (all production types)
        generation_data = client.get_generation_data(country_code, start_date, end_date)
        print(f"Retrieved {len(generation_data)} generation records")
        
        # Combine data
        all_records = load_data + generation_data
        
        if not all_records:
            print(f"No data retrieved for {country_code}")
            # Log no data
            log_sql = """
                INSERT INTO meta_entsoe_ingestion_log (
                    ingestion_date, country_code, start_timestamp, end_timestamp,
                    records_ingested, status
                ) VALUES (%s, %s, %s, %s, %s, %s)
            """
            start_dt = datetime.fromisoformat(start_date)
            end_dt = datetime.fromisoformat(end_date)
            postgres_hook.run(
                log_sql,
                parameters=(
                    datetime.now().date(),
                    country_code,
                    start_dt,
                    end_dt,
                    0,
                    'no_data'
                )
            )
            return {
                'country_code': country_code,
                'start_date': start_date,
                'end_date': end_date,
                'records_ingested': 0,
                'status': 'no_data'
            }
        
        # Insert records into database (batch insert for better performance)
        records_inserted = 0
        batch_size = 100
        for i in range(0, len(all_records), batch_size):
            batch = all_records[i:i + batch_size]
            for record in batch:
                try:
                    sql = """
                        INSERT INTO fact_electricity_timeseries (
                            timestamp_utc, country_code, bidding_zone,
                            production_type, load_mw, generation_mw,
                            resolution, quality_flag
                        ) VALUES (
                            %s, %s, %s, %s, %s, %s, %s, %s
                        )
                        ON CONFLICT DO NOTHING
                    """
                    
                    postgres_hook.run(
                        sql,
                        parameters=(
                            record['timestamp_utc'],
                            record['country_code'],
                            record.get('bidding_zone'),
                            record.get('production_type'),
                            record.get('load_mw'),
                            record.get('generation_mw'),
                            record.get('resolution', 'PT60M'),
                            record.get('quality_flag', 'actual'),
                        )
                    )
                    records_inserted += 1
                except Exception as e:
                    print(f"Error inserting record: {e}")
                    continue
        
        # Log ingestion
        log_sql = """
            INSERT INTO meta_entsoe_ingestion_log (
                ingestion_date, country_code, start_timestamp, end_timestamp,
                records_ingested, status
            ) VALUES (%s, %s, %s, %s, %s, %s)
        """
        
        start_dt = datetime.fromisoformat(start_date)
        end_dt = datetime.fromisoformat(end_date)
        
        postgres_hook.run(
            log_sql,
            parameters=(
                datetime.now().date(),
                country_code,
                start_dt,
                end_dt,
                records_inserted,
                'success' if records_inserted > 0 else 'no_data'
            )
        )
        
        print(f"Successfully ingested {records_inserted} records for {country_code} ({start_date} to {end_date})")
        
        return {
            'country_code': country_code,
            'start_date': start_date,
            'end_date': end_date,
            'records_ingested': records_inserted,
            'status': 'success'
        }
        
    except Exception as e:
        print(f"Error ingesting ENTSO-E data for {country_code} ({start_date} to {end_date}): {e}")
        
        # Log failure
        try:
            log_sql = """
                INSERT INTO meta_entsoe_ingestion_log (
                    ingestion_date, country_code, start_timestamp, end_timestamp,
                    records_ingested, status, error_message
                ) VALUES (%s, %s, %s, %s, %s, %s, %s)
            """
            
            start_dt = datetime.fromisoformat(start_date)
            end_dt = datetime.fromisoformat(end_date)
            
            postgres_hook.run(
                log_sql,
                parameters=(
                    datetime.now().date(),
                    country_code,
                    start_dt,
                    end_dt,
                    0,
                    'failed',
                    str(e)[:500]  # Truncate error message
                )
            )
        except Exception as log_error:
            print(f"Error logging failure: {log_error}")
        
        raise AirflowException(f"Failed to ingest ENTSO-E data for {country_code}: {e}")


def ingest_country_chunks(country_code: str, **context) -> Dict[str, Any]:
    """Ingest all date chunks for a country."""
    ti = context['ti']
    task_ids = [f'country_{country_code.lower()}.plan_{country_code.lower()}']
    plan_result = ti.xcom_pull(task_ids=task_ids)
    
    date_chunks = []
    if plan_result and isinstance(plan_result, dict):
        date_chunks = plan_result.get('date_chunks', [])
    
    if not date_chunks:
        return {
            'country_code': country_code,
            'records_ingested': 0,
            'status': 'up_to_date'
        }
    
    total_records = 0
    for chunk in date_chunks:
        result = ingest_date_chunk(
            country_code=country_code,
            start_date=chunk['start'],
            end_date=chunk['end'],
            **context
        )
        total_records += result.get('records_ingested', 0)
    
    return {
        'country_code': country_code,
        'records_ingested': total_records,
        'status': 'success'
    }


# Create task groups for each country
country_groups = []

for country_code in ENTSOE_COUNTRIES:
    country_lower = country_code.lower()
    
    with TaskGroup(f'country_{country_lower}', dag=dag) as country_group:
        # Checkpoint task
        checkpoint_task = PythonOperator(
            task_id=f'checkpoint_{country_lower}',
            python_callable=check_checkpoint,
            op_kwargs={'country_code': country_code},
            dag=dag,
        )
        
        # Planning task
        plan_task = PythonOperator(
            task_id=f'plan_{country_lower}',
            python_callable=plan_ingestion,
            op_kwargs={'country_code': country_code},
            dag=dag,
        )
        
        # Ingestion task
        ingest_task = PythonOperator(
            task_id=f'ingest_{country_lower}',
            python_callable=ingest_country_chunks,
            op_kwargs={'country_code': country_code},
            dag=dag,
        )
        
        # Set dependencies within country group
        checkpoint_task >> plan_task >> ingest_task
    
    country_groups.append(country_group)


# Summary task
def summarize_ingestion(**context):
    """Summarize ingestion results."""
    postgres_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    sql = """
        SELECT 
            country_code,
            COUNT(*) as total_records,
            MAX(timestamp_utc) as latest_timestamp
        FROM fact_electricity_timeseries
        GROUP BY country_code
        ORDER BY country_code
    """
    
    results = postgres_hook.get_records(sql)
    
    print("\n=== ENTSO-E Ingestion Summary ===")
    for row in results:
        print(f"{row[0]}: {row[1]} records, latest: {row[2]}")
    print("================================\n")


summarize_task = PythonOperator(
    task_id='summarize_ingestion',
    python_callable=summarize_ingestion,
    dag=dag,
)

# Set dependencies: all country groups run in parallel, then summary
for country_group in country_groups:
    country_group >> summarize_task