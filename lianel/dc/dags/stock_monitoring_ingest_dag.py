"""
Stock Monitoring – Quote Ingestion DAG (EU markets MVP)

Placeholder DAG for ingesting EU market quotes (e.g. Euronext, XETRA, LSE).
Implement: fetch from chosen provider (Alpha Vantage, IEX, Polygon, FMP, etc.),
normalize to internal schema, store for dashboard and alert evaluation.

Schedule: intraday (e.g. every 15 min during EU session) or as per provider limits.
"""

from datetime import datetime, timedelta
from airflow import DAG
from airflow.operators.python import PythonOperator


def placeholder_ingest(**context):
    """Placeholder: implement EU quote fetch and store."""
    print("Stock monitoring ingest – EU markets MVP. Implement quote fetch and storage.")


default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'start_date': datetime(2026, 1, 1),
    'email_on_failure': False,
    'retries': 1,
    'retry_delay': timedelta(minutes=5),
}

dag = DAG(
    'stock_monitoring_ingest',
    default_args=default_args,
    description='Ingest EU market quotes for stock monitoring (placeholder)',
    schedule='0 8-17 * * 1-5',  # Hourly 08–17 UTC on weekdays (EU session); adjust when implemented
    catchup=False,
    tags=['stock-monitoring', 'eu-markets', 'quotes', 'mvp'],
)

PythonOperator(
    task_id='ingest_eu_quotes',
    python_callable=placeholder_ingest,
    dag=dag,
)
