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

# EU27 country codes
EU27_COUNTRIES = [
    'AT', 'BE', 'BG', 'CY', 'CZ', 'DE', 'DK', 'EE', 'EL', 'ES', 'FI', 'FR',
    'HR', 'HU', 'IE', 'IT', 'LT', 'LU', 'LV', 'MT', 'NL', 'PL', 'PT', 'RO',
    'SE', 'SI', 'SK'
]

# Countries with ENTSO-E data availability (subset of EU27)
ENTSOE_COUNTRIES = [
    'AT', 'BE', 'BG', 'CZ', 'DE', 'DK', 'EE', 'ES', 'FI', 'FR',
    'HR', 'HU', 'IE', 'IT', 'LT', 'LU', 'LV', 'NL', 'PL', 'PT', 'RO',
    'SE', 'SI', 'SK'
]


def get_last_ingestion_date(postgres_hook: PostgresHook, country_code: str) -> Optional[datetime]:
    """Get the last successful ingestion date for a country."""
    sql = """
        SELECT MAX(end_timestamp) 
        FROM meta_entsoe_ingestion_log 
        WHERE country_code = %s AND status = 'success'
    """
    try:
        result = postgres_hook.get_first(sql, parameters=(country_code,))
        if result and result[0]:
            return result[0]
    except Exception as e:
        print(f"Error getting last ingestion date for {country_code}: {e}")
    return None


def ingest_entsoe_data(
    country_code: str,
    start_date: str,
    end_date: str,
    api_token: Optional[str] = None,
    **context
) -> Dict[str, Any]:
    """
    Ingest ENTSO-E data for a country and date range.
    
    Args:
        country_code: ISO 2-letter country code
        start_date: Start date in format 'YYYY-MM-DD'
        end_date: End date in format 'YYYY-MM-DD'
        api_token: Optional ENTSO-E API token
    """
    postgres_hook = PostgresHook(postgres_conn_id='postgres_default')
    
    # Initialize ENTSO-E client
    client = ENTSOEClient(api_token=api_token)
    
    # Get combined load and generation data
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
            return {
                'country_code': country_code,
                'records_ingested': 0,
                'status': 'no_data'
            }
        
        # Insert records into database
        records_inserted = 0
        for record in all_records:
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
            ) VALUES (
                %s, %s, %s, %s, %s, %s
            )
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
        
        print(f"Successfully ingested {records_inserted} records for {country_code}")
        
        return {
            'country_code': country_code,
            'records_ingested': records_inserted,
            'status': 'success'
        }
        
    except Exception as e:
        print(f"Error ingesting ENTSO-E data for {country_code}: {e}")
        
        # Log failure
        try:
            log_sql = """
                INSERT INTO meta_entsoe_ingestion_log (
                    ingestion_date, country_code, start_timestamp, end_timestamp,
                    records_ingested, status, error_message
                ) VALUES (
                    %s, %s, %s, %s, %s, %s, %s
                )
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


def ingest_country_data(country_code: str, **context) -> Dict[str, Any]:
    """Ingest ENTSO-E data for a single country."""
    postgres_hook = PostgresHook(postgres_conn_id='postgres_default')
    
    # Get last ingestion date
    last_date = get_last_ingestion_date(postgres_hook, country_code)
    
    # Determine date range
    if last_date:
        # Start from day after last ingestion
        start_date = (last_date + timedelta(days=1)).strftime('%Y-%m-%d')
    else:
        # Start from 30 days ago (initial load)
        start_date = (datetime.now() - timedelta(days=30)).strftime('%Y-%m-%d')
    
    # End date is yesterday (ENTSO-E data available with 1-day delay)
    end_date = (datetime.now() - timedelta(days=1)).strftime('%Y-%m-%d')
    
    if start_date >= end_date:
        print(f"No new data to ingest for {country_code} (last: {last_date})")
        return {
            'country_code': country_code,
            'records_ingested': 0,
            'status': 'up_to_date'
        }
    
    # Get API token from Airflow Variable (if set)
    from airflow.models import Variable
    api_token = Variable.get("ENTSOE_API_TOKEN", default_var=None)
    
    # Process in 7-day chunks to avoid large requests
    start_dt = datetime.fromisoformat(start_date)
    end_dt = datetime.fromisoformat(end_date)
    
    total_records = 0
    
    while start_dt < end_dt:
        chunk_end = min(start_dt + timedelta(days=7), end_dt)
        
        result = ingest_entsoe_data(
            country_code=country_code,
            start_date=start_dt.strftime('%Y-%m-%d'),
            end_date=chunk_end.strftime('%Y-%m-%d'),
            api_token=api_token,
            **context
        )
        
        total_records += result.get('records_ingested', 0)
        start_dt = chunk_end
    
    return {
        'country_code': country_code,
        'records_ingested': total_records,
        'status': 'success'
    }


# Create task group for country ingestion
with TaskGroup('ingest_countries', dag=dag) as ingest_group:
    country_tasks = []
    
    for country_code in ENTSOE_COUNTRIES:
        task = PythonOperator(
            task_id=f'ingest_{country_code.lower()}',
            python_callable=ingest_country_data,
            op_kwargs={'country_code': country_code},
            dag=dag,
        )
        country_tasks.append(task)

# Summary task
def summarize_ingestion(**context):
    """Summarize ingestion results."""
    postgres_hook = PostgresHook(postgres_conn_id='postgres_default')
    
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

# Set dependencies
ingest_group >> summarize_task
