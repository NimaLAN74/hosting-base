"""
Stock Service - Simulator replay DAG

Runs the multi-exchange replay simulator on a schedule so we continuously detect
bias, process gaps, and missing data issues from realistic execution replay.
"""

import json
import os
from datetime import datetime, timedelta
from urllib import request

from airflow import DAG
from airflow.providers.standard.operators.python import PythonOperator


def run_simulator_replay(**context):
    base_url = os.getenv("STOCK_MONITORING_INTERNAL_URL", "http://lianel-stock-service:3003")
    days = int(os.getenv("SIM_REPLAY_DAYS", "7"))
    payload = {
        "days": max(5, min(days, 30)),
        "top": int(os.getenv("SIM_REPLAY_TOP", "16")),
        "quantile": float(os.getenv("SIM_REPLAY_QUANTILE", "0.2")),
        "short_enabled": os.getenv("SIM_REPLAY_SHORT_ENABLED", "true").lower() in ("1", "true", "yes"),
        "initial_capital_usd": float(os.getenv("SIM_REPLAY_INITIAL_CAPITAL_USD", "100")),
        "reinvest_profit": os.getenv("SIM_REPLAY_REINVEST", "true").lower() in ("1", "true", "yes"),
        "replay_delay_ms": int(os.getenv("SIM_REPLAY_DELAY_MS", "0")),
    }
    url = f"{base_url.rstrip('/')}/api/v1/stock-service/sim/runs"
    body = json.dumps(payload).encode("utf-8")
    req = request.Request(
        url=url,
        method="POST",
        data=body,
        headers={"Content-Type": "application/json", "Accept": "application/json"},
    )
    with request.urlopen(req, timeout=90) as response:  # nosec B310 - trusted internal endpoint
        status = response.status
        raw = response.read().decode("utf-8")
    if status < 200 or status >= 300:
        raise RuntimeError(f"Simulator run trigger failed: HTTP {status} - {raw[:500]}")
    data = json.loads(raw)
    run_id = data.get("run_id", "<unknown>")
    print(f"Simulator run started: {run_id}")


default_args = {
    "owner": "lianel",
    "depends_on_past": False,
    "start_date": datetime(2026, 1, 1),
    "email_on_failure": False,
    "retries": 1,
    "retry_delay": timedelta(minutes=5),
}

dag = DAG(
    "stock_monitoring_simulator_replay",
    default_args=default_args,
    description="Trigger stock replay simulator to find model/process/data gaps",
    schedule="30 2 * * 1-5",  # Weekdays after most daily data is available
    catchup=False,
    tags=["stock-service", "simulator", "bias-detection"],
)

PythonOperator(
    task_id="run_simulator_replay",
    python_callable=run_simulator_replay,
    dag=dag,
)

