"""
Stock Monitoring – Alert Evaluation DAG

Runs after quote ingest and triggers backend alert evaluation.
"""

from datetime import datetime, timedelta
import json
import os
from urllib import request

from airflow import DAG
from airflow.operators.python import PythonOperator
from airflow.sensors.external_task import ExternalTaskSensor


def trigger_alert_evaluation(**context):
    base_url = os.getenv("STOCK_MONITORING_INTERNAL_URL", "http://lianel-stock-monitoring-service:3003")
    url = f"{base_url.rstrip('/')}/internal/alerts/evaluate"
    req = request.Request(url=url, method="POST")
    with request.urlopen(req, timeout=30) as response:  # nosec B310 - trusted internal endpoint
        status = response.status
        body = response.read().decode("utf-8")
    if status < 200 or status >= 300:
        raise RuntimeError(f"Alert evaluation failed: HTTP {status} - {body}")
    payload = json.loads(body or "{}")
    print(f"Alert evaluation result: {payload}")


default_args = {
    "owner": "lianel",
    "depends_on_past": False,
    "start_date": datetime(2026, 1, 1),
    "email_on_failure": False,
    "retries": 1,
    "retry_delay": timedelta(minutes=5),
}

dag = DAG(
    "stock_monitoring_alerts",
    default_args=default_args,
    description="Trigger stock alert evaluation after quote ingest",
    schedule="5 8-17 * * 1-5",  # five minutes after ingest schedule
    catchup=False,
    tags=["stock-monitoring", "alerts", "eu-markets", "mvp"],
)

wait_for_ingest = ExternalTaskSensor(
    task_id="wait_for_stock_ingest",
    external_dag_id="stock_monitoring_ingest",
    external_task_id="ingest_eu_quotes",
    # Alerts run at :05, while ingest runs at :00; align sensor lookup to previous run.
    execution_delta=timedelta(minutes=5),
    allowed_states=["success"],
    failed_states=["failed", "upstream_failed"],
    mode="reschedule",
    timeout=60 * 30,
    poke_interval=60,
    dag=dag,
)

evaluate_alerts = PythonOperator(
    task_id="evaluate_stock_alerts",
    python_callable=trigger_alert_evaluation,
    dag=dag,
)

wait_for_ingest >> evaluate_alerts
