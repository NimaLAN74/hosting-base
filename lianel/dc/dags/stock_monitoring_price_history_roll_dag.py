"""
Stock Service – Daily price history roll (EOD)

Calls POST /internal/price-history/roll-daily to aggregate intraday cache
into price_history_daily (OHLC) and clear the intraday table for past days.
Run once per day after market close (e.g. 00:05 UTC).
"""

import os
from datetime import datetime, timedelta
from urllib import request, error

from airflow import DAG
from airflow.exceptions import AirflowSkipException
from airflow.providers.standard.operators.python import PythonOperator


def roll_daily_price_history(**context):
    """POST /internal/price-history/roll-daily to roll intraday -> daily OHLC and clear cache."""
    base_url = os.getenv("STOCK_MONITORING_INTERNAL_URL", "http://lianel-stock-service:3003")
    url = f"{base_url.rstrip('/')}/internal/price-history/roll-daily"
    req = request.Request(url=url, method="POST", data=b"")
    req.add_header("Content-Type", "application/json")
    try:
        with request.urlopen(req, timeout=120) as response:  # nosec B310 - trusted internal endpoint
            status = response.status
            body = response.read().decode("utf-8")
    except error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace") if hasattr(e, "read") else str(e)
        if e.code >= 500:
            raise AirflowSkipException(f"Stock service transient HTTP {e.code}: {body[:500]}")
        raise RuntimeError(f"Roll daily failed: HTTP {e.code} - {body[:500]}")
    except error.URLError as e:
        raise AirflowSkipException(f"Stock service unavailable (network): {e}")
    if status < 200 or status >= 300:
        if status >= 500:
            raise AirflowSkipException(f"Stock service transient HTTP {status}: {body[:500]}")
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
    tags=["stock-service", "price-history", "eod", "mvp"],
)

PythonOperator(
    task_id="roll_daily",
    python_callable=roll_daily_price_history,
    dag=dag,
)
