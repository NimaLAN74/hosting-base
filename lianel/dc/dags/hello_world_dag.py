from datetime import datetime, timedelta
from airflow import DAG
from airflow.operators.python import PythonOperator

def print_hello():
    print("Hello from Lianel World!")
    return "Hello World task completed"

def print_date():
    print(f"Current date and time: {datetime.now()}")
    return "Date task completed"

default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'start_date': datetime(2025, 11, 27),
    'email_on_failure': False,
    'email_on_retry': False,
    'retries': 1,
    'retry_delay': timedelta(minutes=5),
}

dag = DAG(
    'hello_world_dag',
    default_args=default_args,
    description='A simple hello world DAG',
    schedule=timedelta(days=1),
    catchup=False,
    tags=['example', 'lianel'],
)

hello_task = PythonOperator(
    task_id='print_hello',
    python_callable=print_hello,
    dag=dag,
)

date_task = PythonOperator(
    task_id='print_date',
    python_callable=print_date,
    dag=dag,
)

hello_task >> date_task
