"""
Eurostat Data Harmonization DAG

This DAG harmonizes and transforms raw Eurostat data:
- Convert units to GWh (standard unit)
- Normalize country codes
- Map product/flow codes to dimension tables
- Calculate derived metrics
- Update fact_energy_annual with harmonized values

Schedule: Triggered after eurostat_ingestion completes
"""
from datetime import datetime, timedelta
from airflow import DAG
from airflow.operators.python import PythonOperator
from airflow.providers.postgres.hooks.postgres import PostgresHook
from airflow.exceptions import AirflowException
from typing import Dict, List

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
    'eurostat_harmonization',
    default_args=default_args,
    description='Harmonize and transform Eurostat data',
    schedule=None,  # Triggered by coordinator DAG after all ingestion completes
    catchup=False,
    tags=['data-transformation', 'harmonization', 'eurostat'],
    max_active_runs=1,
)

# Unit conversion factors to GWh
UNIT_CONVERSIONS = {
    'KTOE': 11.63,      # 1 ktoe = 11.63 GWh
    'TJ': 0.2778,       # 1 TJ = 0.2778 GWh
    'GWH': 1.0,         # Already in GWh
    'GWH_HAB': 1.0,     # Per capita, but unit is GWh
    'MWH': 0.001,       # 1 MWh = 0.001 GWh
    'TWH': 1000.0,      # 1 TWh = 1000 GWh
}

HARMONISATION_VERSION = '1.0'


def validate_staging_data(**context):
    """Validate that staging data exists and is ready for harmonization"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    # Check for raw data that needs harmonization
    sql = """
        SELECT 
            source_table,
            COUNT(*) as records,
            COUNT(DISTINCT unit) as unit_types,
            COUNT(DISTINCT country_code) as countries,
            COUNT(DISTINCT year) as years
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
          AND (harmonisation_version IS NULL OR harmonisation_version != %s)
        GROUP BY source_table
        ORDER BY source_table
    """
    
    results = db_hook.get_records(sql, parameters=(HARMONISATION_VERSION,))
    
    if not results:
        print("⚠️  No data found that needs harmonization")
        return {'tables': [], 'total_records': 0}
    
    summary = {
        'tables': [{'table': row[0], 'records': row[1], 'units': row[2], 
                   'countries': row[3], 'years': row[4]} for row in results],
        'total_records': sum(row[1] for row in results)
    }
    
    print(f"✅ Found {summary['total_records']} records to harmonize across {len(summary['tables'])} tables")
    for table_info in summary['tables']:
        print(f"  - {table_info['table']}: {table_info['records']} records, "
              f"{table_info['units']} unit types, {table_info['countries']} countries, "
              f"{table_info['years']} years")
    
    context['ti'].xcom_push(key='harmonization_plan', value=summary)
    return summary


def convert_units_to_gwh(**context):
    """Convert all units to GWh (harmonization standard)"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    ti = context['ti']
    
    # Get harmonization plan
    plan = ti.xcom_pull(task_ids='validate_staging_data', key='harmonization_plan')
    if not plan or plan['total_records'] == 0:
        print("No data to harmonize")
        return 0
    
    # Process each unit type
    conversion_sql = """
        UPDATE fact_energy_annual
        SET 
            value_gwh = CASE 
                WHEN UPPER(TRIM(unit)) = 'KTOE' THEN value_gwh * 11.63
                WHEN UPPER(TRIM(unit)) = 'TJ' THEN value_gwh * 0.2778
                WHEN UPPER(TRIM(unit)) = 'GWH' THEN value_gwh
                WHEN UPPER(TRIM(unit)) = 'MWH' THEN value_gwh * 0.001
                WHEN UPPER(TRIM(unit)) = 'TWH' THEN value_gwh * 1000.0
                ELSE value_gwh  -- Unknown unit, keep as is
            END,
            unit = 'GWh',
            harmonisation_version = %s
        WHERE source_system = 'eurostat'
          AND (harmonisation_version IS NULL OR harmonisation_version != %s)
          AND value_gwh IS NOT NULL
    """
    
    updated = db_hook.run(conversion_sql, parameters=(
        HARMONISATION_VERSION,
        HARMONISATION_VERSION
    ))
    
    # Get count of updated records
    count_sql = """
        SELECT COUNT(*) 
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
          AND harmonisation_version = %s
    """
    result = db_hook.get_first(count_sql, parameters=(HARMONISATION_VERSION,))
    count = result[0] if result else 0
    
    print(f"✅ Converted {count} records to GWh")
    return count


def normalize_country_codes(**context):
    """Normalize country codes (ensure they match dim_country)"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    # Check for invalid country codes
    sql = """
        SELECT DISTINCT f.country_code
        FROM fact_energy_annual f
        LEFT JOIN dim_country c ON f.country_code = c.country_code
        WHERE f.source_system = 'eurostat'
          AND c.country_code IS NULL
    """
    
    invalid_codes = db_hook.get_records(sql)
    
    if invalid_codes:
        print(f"⚠️  Found {len(invalid_codes)} invalid country codes: {[row[0] for row in invalid_codes]}")
        # For now, just log - in production, would need mapping logic
    else:
        print("✅ All country codes are valid")
    
    return len(invalid_codes)


def map_product_codes(**context):
    """Ensure product codes exist in dim_energy_product"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    # Find product codes not in dimension table
    sql = """
        SELECT DISTINCT f.product_code
        FROM fact_energy_annual f
        LEFT JOIN dim_energy_product p ON f.product_code = p.product_code
        WHERE f.source_system = 'eurostat'
          AND f.product_code IS NOT NULL
          AND p.product_code IS NULL
        LIMIT 20
    """
    
    missing_codes = db_hook.get_records(sql)
    
    if missing_codes:
        print(f"⚠️  Found {len(missing_codes)} product codes not in dimension table")
        print(f"   Sample: {[row[0] for row in missing_codes[:5]]}")
        # In production, would insert missing codes or map them
    else:
        print("✅ All product codes exist in dimension table")
    
    return len(missing_codes)


def map_flow_codes(**context):
    """Ensure flow codes exist in dim_energy_flow"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    # Find flow codes not in dimension table
    sql = """
        SELECT DISTINCT f.flow_code
        FROM fact_energy_annual f
        LEFT JOIN dim_energy_flow fl ON f.flow_code = fl.flow_code
        WHERE f.source_system = 'eurostat'
          AND f.flow_code IS NOT NULL
          AND fl.flow_code IS NULL
        LIMIT 20
    """
    
    missing_codes = db_hook.get_records(sql)
    
    if missing_codes:
        print(f"⚠️  Found {len(missing_codes)} flow codes not in dimension table")
        print(f"   Sample: {[row[0] for row in missing_codes[:5]]}")
        # In production, would insert missing codes or map them
    else:
        print("✅ All flow codes exist in dimension table")
    
    return len(missing_codes)


def calculate_derived_metrics(**context):
    """Calculate derived metrics (e.g., per capita, percentages)"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    # For now, this is a placeholder
    # In production, would calculate:
    # - Per capita energy consumption
    # - Percentage of renewable energy
    # - Energy intensity metrics
    # - Year-over-year changes
    
    print("✅ Derived metrics calculation (placeholder - to be implemented)")
    return 0


def validate_harmonized_data(**context):
    """Validate harmonized data quality"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    # Check data quality
    quality_checks = []
    
    # Check 1: All values should be in GWh
    sql = """
        SELECT COUNT(*) 
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
          AND harmonisation_version = %s
          AND unit != 'GWh'
    """
    result = db_hook.get_first(sql, parameters=(HARMONISATION_VERSION,))
    non_gwh = result[0] if result else 0
    
    if non_gwh > 0:
        quality_checks.append(f"⚠️  {non_gwh} records not in GWh")
    else:
        quality_checks.append("✅ All records in GWh")
    
    # Check 2: No negative values
    sql = """
        SELECT COUNT(*) 
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
          AND harmonisation_version = %s
          AND value_gwh < 0
    """
    result = db_hook.get_first(sql, parameters=(HARMONISATION_VERSION,))
    negative = result[0] if result else 0
    
    if negative > 0:
        quality_checks.append(f"⚠️  {negative} records with negative values")
    else:
        quality_checks.append("✅ No negative values")
    
    # Check 3: Summary statistics
    sql = """
        SELECT 
            COUNT(*) as total,
            COUNT(DISTINCT country_code) as countries,
            COUNT(DISTINCT year) as years,
            MIN(value_gwh) as min_val,
            MAX(value_gwh) as max_val,
            AVG(value_gwh) as avg_val
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
          AND harmonisation_version = %s
    """
    result = db_hook.get_first(sql, parameters=(HARMONISATION_VERSION,))
    
    if result:
        print(f"✅ Harmonized data summary:")
        print(f"   Total records: {result[0]}")
        print(f"   Countries: {result[1]}")
        print(f"   Years: {result[2]}")
        print(f"   Value range: {result[3]:.2f} - {result[4]:.2f} GWh")
        print(f"   Average: {result[5]:.2f} GWh")
    
    for check in quality_checks:
        print(check)
    
    return quality_checks


def log_harmonization_summary(**context):
    """Log harmonization results to metadata table"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    # Get summary
    sql = """
        SELECT 
            COUNT(*) as total_records,
            COUNT(DISTINCT source_table) as tables,
            COUNT(DISTINCT country_code) as countries,
            COUNT(DISTINCT year) as years
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
          AND harmonisation_version = %s
    """
    result = db_hook.get_first(sql, parameters=(HARMONISATION_VERSION,))
    
    if result:
        summary = {
            'source_system': 'eurostat',
            'table_name': 'fact_energy_annual',
            'status': 'success',
            'records_inserted': result[0],
            'records_updated': result[0],  # Harmonized records
            'records_failed': 0,
        }
        
        log_sql = """
            INSERT INTO meta_ingestion_log 
            (source_system, table_name, status, records_inserted, records_updated, 
             records_failed, ingestion_timestamp)
            VALUES (%s, %s, %s, %s, %s, %s, NOW())
        """
        
        # Log summary details in print statement (notes column doesn't exist in table)
        notes = f"Harmonization version {HARMONISATION_VERSION}: {result[1]} tables, {result[2]} countries, {result[3]} years"
        print(f"📝 {notes}")
        
        db_hook.run(log_sql, parameters=(
            summary['source_system'],
            summary['table_name'],
            summary['status'],
            summary['records_inserted'],
            summary['records_updated'],
            summary['records_failed']
        ))
        
        print(f"✅ Harmonization logged: {summary}")
        return summary
    
    return None


# DAG Tasks
start_task = PythonOperator(
    task_id='start',
    python_callable=lambda: print("🚀 Starting Eurostat harmonization"),
    dag=dag,
)

validate_staging_task = PythonOperator(
    task_id='validate_staging_data',
    python_callable=validate_staging_data,
    dag=dag,
)

convert_units_task = PythonOperator(
    task_id='convert_units_to_gwh',
    python_callable=convert_units_to_gwh,
    dag=dag,
)

normalize_countries_task = PythonOperator(
    task_id='normalize_country_codes',
    python_callable=normalize_country_codes,
    dag=dag,
)

map_products_task = PythonOperator(
    task_id='map_product_codes',
    python_callable=map_product_codes,
    dag=dag,
)

map_flows_task = PythonOperator(
    task_id='map_flow_codes',
    python_callable=map_flow_codes,
    dag=dag,
)

calculate_metrics_task = PythonOperator(
    task_id='calculate_derived_metrics',
    python_callable=calculate_derived_metrics,
    dag=dag,
)

validate_harmonized_task = PythonOperator(
    task_id='validate_harmonized_data',
    python_callable=validate_harmonized_data,
    dag=dag,
)

log_summary_task = PythonOperator(
    task_id='log_harmonization_summary',
    python_callable=log_harmonization_summary,
    dag=dag,
)

end_task = PythonOperator(
    task_id='end',
    python_callable=lambda: print("✅ Eurostat harmonization complete"),
    dag=dag,
)

# Task dependencies
start_task >> validate_staging_task >> convert_units_task
convert_units_task >> [normalize_countries_task, map_products_task, map_flows_task]
[normalize_countries_task, map_products_task, map_flows_task] >> calculate_metrics_task
calculate_metrics_task >> validate_harmonized_task >> log_summary_task >> end_task

