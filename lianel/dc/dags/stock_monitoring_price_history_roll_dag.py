"""
Stock Monitoring – Daily price history roll (EOD)

Calls POST /internal/price-history/roll-daily to aggregate intraday cache
into price_history_daily (OHLC) and clear the intraday table for past days.
Run once per day after market close (e.g. 00:05 UTC).
"""

import os
from datetime import datetime, timedelta
from urllib import request

from airflow import DAG
from airflow.operators.python import PythonOperator


def roll_daily_price_history(**context):
    """POST /internal/price-history/roll-daily to roll intraday -> daily OHLC and clear cache."""
    base_url = os.getenv("STOCK_MONITORING_INTERNAL_URL", "http://lianel-stock-monitoring-service:3003")
    url = f"{base_url.rstrip('/')}/internal/price-history/roll-daily"
    req = request.Request(url=url, method="POST", data=b"")
    req.add_header("Content-Type", "application/json")
    with request.urlopen(req, timeout=120) as response:  # nosec B310 - trusted internal endpoint
        status = response.status
        body = response.read().decode("utf-8")
    if status < 200 or status >= 300:
        raise RuntimeError(f"Roll daily failed: HTTP {status} - {body[:500]}")
    try:
        import json
        data = json.loads(body)
        print(f"Roll complete: daily_rows={data.get('daily_rows', 0)}, intraday_deleted={data.get('intraday_deleted', 0)}")
    except Exception:
        print(f"Roll complete: HTTP {status}")


default_args = {
    "owner": "lianel",
    "depends_on_past": False,
    "start_date": datetime(2026, 1, 1),
    "email_on_failure": False,
    "retries": 2,
    "retry_delay": timedelta(minutes=5),
}

dag = DAG(
    "stock_monitoring_price_history_roll",
    default_args=default_args,
    description="Roll intraday price cache into daily OHLC and clear cache (EOD)",
    schedule="5 0 * * *",  # 00:05 UTC daily
    catchup=False,
    tags=["stock-monitoring", "price-history", "eod", "mvp"],
)

PythonOperator(
    task_id="roll_daily",
    python_callable=roll_daily_price_history,
    dag=dag,
)
