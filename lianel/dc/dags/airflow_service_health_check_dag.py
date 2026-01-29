"""
Airflow Service Health Check DAG

Periodically verifies that core Airflow services and functions are healthy:
- API server (REST API /version)
- Scheduler (internal health HTTP server)
- Metadata database (SELECT 1 via config DB URL; works in worker)

Used for monitoring and alerting when any component is down or slow.
Schedule: Every 10 minutes.
"""
from datetime import datetime, timedelta
from airflow import DAG
from airflow.operators.python import PythonOperator
from airflow.exceptions import AirflowException
from sqlalchemy import text

default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'start_date': datetime(2026, 1, 1),
    'email_on_failure': False,
    'email_on_retry': False,
    'retries': 2,
    'retry_delay': timedelta(minutes=2),
    'execution_timeout': timedelta(minutes=5),
}

dag = DAG(
    'airflow_service_health_check',
    default_args=default_args,
    description='Health check of Airflow API server, scheduler, and metadata DB',
    schedule='*/10 * * * *',  # Every 10 minutes
    catchup=False,
    tags=['health-check', 'monitoring', 'airflow'],
    max_active_runs=1,
)


def check_api_server(**context):
    """Verify API server responds on /api/v2/version (same as compose healthcheck)."""
    import requests
    url = 'http://airflow-apiserver:8080/api/v2/version'
    timeout = 10
    try:
        resp = requests.get(url, timeout=timeout)
        resp.raise_for_status()
        data = resp.json()
        context['ti'].xcom_push(key='version', value=data.get('version', 'unknown'))
        context['ti'].xcom_push(key='response_time_ms', value=resp.elapsed.total_seconds() * 1000)
        return {'status': 'ok', 'version': data.get('version'), 'status_code': resp.status_code}
    except requests.RequestException as e:
        raise AirflowException(f"API server health check failed: {e}") from e


def check_scheduler_health(**context):
    """Verify scheduler health HTTP server responds (AIRFLOW__SCHEDULER__ENABLE_HEALTH_CHECK)."""
    import requests
    url = 'http://airflow-scheduler:8974/health'
    timeout = 10
    try:
        resp = requests.get(url, timeout=timeout)
        resp.raise_for_status()
        context['ti'].xcom_push(key='response_time_ms', value=resp.elapsed.total_seconds() * 1000)
        return {'status': 'ok', 'status_code': resp.status_code}
    except requests.RequestException as e:
        raise AirflowException(f"Scheduler health check failed: {e}") from e


def check_database(**context):
    """Verify metadata database is reachable (SELECT 1). Uses DB URL from config so it works in worker; avoids ORM/session (restricted in Airflow 3)."""
    from airflow.configuration import conf
    from sqlalchemy import create_engine

    try:
        uri = conf.get("database", "sql_alchemy_conn")
        engine = create_engine(uri)
        with engine.connect() as conn:
            conn.execute(text("SELECT 1"))
        return {"status": "ok"}
    except Exception as e:
        raise AirflowException(f"Metadata database health check failed: {e}") from e


# Task: API server health
health_check_api_server = PythonOperator(
    task_id='health_check_api_server',
    python_callable=check_api_server,
    dag=dag,
)

# Task: Scheduler health
health_check_scheduler = PythonOperator(
    task_id='health_check_scheduler',
    python_callable=check_scheduler_health,
    dag=dag,
)

# Task: Metadata database (SELECT 1 via config; works in worker, unlike airflow db check CLI)
health_check_database = PythonOperator(
    task_id='health_check_database',
    python_callable=check_database,
    dag=dag,
)

# All checks run in parallel (no dependencies); failure of any task fails the DAG run
