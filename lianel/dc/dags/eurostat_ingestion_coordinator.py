"""
Eurostat Ingestion Coordinator DAG

This DAG coordinates the execution of all table-specific ingestion DAGs.
It uses ExternalTaskSensor to wait for all table DAGs to complete,
then triggers harmonization.

Schedule: Weekly (every Sunday at 02:00 UTC) - same as table DAGs
"""
from datetime import datetime, timedelta
from airflow import DAG
from airflow.operators.python import PythonOperator
from airflow.sensors.external_task import ExternalTaskSensor
from airflow.operators.trigger_dagrun import TriggerDagRunOperator
from airflow.providers.postgres.hooks.postgres import PostgresHook

default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'start_date': datetime(2026, 1, 1),
    'email_on_failure': False,
    'email_on_retry': False,
    'retries': 1,
    'retry_delay': timedelta(minutes=5),
    'execution_timeout': timedelta(hours=1),
}

dag = DAG(
    'eurostat_ingestion_coordinator',
    default_args=default_args,
    description='Coordinate Eurostat table ingestion DAGs and trigger harmonization',
    schedule='0 2 * * 0',  # Every Sunday at 02:00 UTC
    catchup=False,
    tags=['data-ingestion', 'eurostat', 'coordinator'],
    max_active_runs=1,
)

# Table DAGs to wait for
TABLE_DAGS = [
    'eurostat_ingestion_nrg_bal_s',
    'eurostat_ingestion_nrg_cb_e',
    'eurostat_ingestion_nrg_ind_eff',
    'eurostat_ingestion_nrg_ind_ren',
]


def check_all_tables_complete(**context):
    """Verify all table DAGs have completed successfully"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    # Check ingestion logs for today
    sql = """
        SELECT 
            table_name,
            status,
            records_inserted,
            ingestion_timestamp
        FROM meta_ingestion_log
        WHERE source_system = 'eurostat'
          AND ingestion_timestamp >= CURRENT_DATE
          AND table_name IN ('nrg_bal_s', 'nrg_cb_e', 'nrg_ind_eff', 'nrg_ind_ren')
        ORDER BY table_name
    """
    
    results = db_hook.get_records(sql)
    
    if not results:
        print("⚠️  No ingestion logs found for today")
        return False
    
    print("📊 Table ingestion status:")
    all_success = True
    for row in results:
        table, status, records, timestamp = row
        status_icon = "✅" if status == 'success' else "❌"
        print(f"   {status_icon} {table}: {status} ({records} records)")
        if status != 'success':
            all_success = False
    
    if all_success:
        print("✅ All table DAGs completed successfully")
    else:
        print("⚠️  Some table DAGs did not complete successfully")
    
    return all_success


# Trigger harmonization task - using TriggerDagRunOperator instead of function


# Start task
start_task = PythonOperator(
    task_id='start',
    python_callable=lambda: print("🚀 Starting ingestion coordinator"),
    dag=dag,
)

# Wait for all table DAGs to complete
table_sensors = []
for table_dag_id in TABLE_DAGS:
    sensor = ExternalTaskSensor(
        task_id=f'wait_for_{table_dag_id}',
        external_dag_id=table_dag_id,
        external_task_id='end',  # Wait for the 'end' task of each table DAG
        timeout=timedelta(hours=4),  # Max wait time
        poke_interval=300,  # Check every 5 minutes
        mode='reschedule',  # Don't hold worker slot
        dag=dag,
    )
    table_sensors.append(sensor)

# Verify completion
check_completion_task = PythonOperator(
    task_id='check_all_tables_complete',
    python_callable=check_all_tables_complete,
    dag=dag,
)

# Trigger harmonization DAG
trigger_harmonization_task = TriggerDagRunOperator(
    task_id='trigger_harmonization',
    trigger_dag_id='eurostat_harmonization',
    conf={'triggered_by': 'ingestion_coordinator'},
    wait_for_completion=False,  # Don't wait, just trigger
    poke_interval=60,
    dag=dag,
)

end_task = PythonOperator(
    task_id='end',
    python_callable=lambda: print("✅ Ingestion coordinator complete"),
    dag=dag,
)

# DAG dependencies
start_task >> table_sensors
table_sensors >> check_completion_task
check_completion_task >> trigger_harmonization_task
trigger_harmonization_task >> end_task
