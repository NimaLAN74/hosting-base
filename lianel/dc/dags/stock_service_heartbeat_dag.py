"""
Stock service connectivity check (current IBKR-backed stack).

Calls GET /health and GET /api/v1/watchlist on the internal stock-service URL.
The old Yahoo/Finnhub ingest, roll-daily, alerts/evaluate, and symbol-refresh
DAGs were removed; those routes are not implemented on the current backend.

Env (first match wins):
  STOCK_SERVICE_INTERNAL_URL, then STOCK_MONITORING_INTERNAL_URL,
  default http://lianel-stock-service:3003
"""

import json
import os
from datetime import datetime, timedelta
from urllib import error, request

from airflow import DAG
from airflow.exceptions import AirflowSkipException

try:
    from airflow.providers.standard.operators.python import PythonOperator
except ImportError:
    from airflow.operators.python import PythonOperator


def _stock_base_url() -> str:
    return (
        os.getenv("STOCK_SERVICE_INTERNAL_URL")
        or os.getenv("STOCK_MONITORING_INTERNAL_URL")
        or "http://lianel-stock-service:3003"
    ).rstrip("/")


def ping_stock_service(**context):
    base = _stock_base_url()
    health_url = f"{base}/health"
    watch_url = f"{base}/api/v1/watchlist"

    try:
        with request.urlopen(health_url, timeout=30) as r:  # nosec B310
            status = r.status
            body = r.read().decode("utf-8", errors="replace")
    except error.HTTPError as e:
        if e.code >= 500:
            raise AirflowSkipException(f"Stock service health HTTP {e.code}")
        raise RuntimeError(f"Stock service health failed: HTTP {e.code}")
    except error.URLError as e:
        raise AirflowSkipException(f"Stock service unreachable: {e}")

    if status < 200 or status >= 300:
        if status >= 500:
            raise AirflowSkipException(f"Stock service health HTTP {status}")
        raise RuntimeError(f"Stock service health failed: HTTP {status}")
    if "ok" not in body.lower():
        print(f"Health body (unexpected): {body[:200]!r}")

    try:
        with request.urlopen(watch_url, timeout=60) as r:  # nosec B310
            wstatus = r.status
            wbody = r.read().decode("utf-8", errors="replace")
    except error.HTTPError as e:
        if e.code >= 500:
            raise AirflowSkipException(f"Watchlist HTTP {e.code}")
        raise RuntimeError(f"Watchlist failed: HTTP {e.code}")
    except error.URLError as e:
        raise AirflowSkipException(f"Watchlist unreachable: {e}")

    if wstatus < 200 or wstatus >= 300:
        if wstatus >= 500:
            raise AirflowSkipException(f"Watchlist HTTP {wstatus}")
        raise RuntimeError(f"Watchlist failed: HTTP {wstatus}")

    data = json.loads(wbody)
    symbols = data.get("symbols") if isinstance(data, dict) else None
    n = len(symbols) if isinstance(symbols, list) else 0
    print(f"Stock service OK: health={body.strip()!r}, watchlist_symbols={n}")


default_args = {
    "owner": "lianel",
    "depends_on_past": False,
    "start_date": datetime(2026, 1, 1),
    "email_on_failure": False,
    "retries": 1,
    "retry_delay": timedelta(minutes=5),
}

dag = DAG(
    "stock_service_heartbeat",
    default_args=default_args,
    description="Verify stock-service /health and /api/v1/watchlist (internal URL)",
    schedule="0 * * * *",  # hourly
    catchup=False,
    tags=["stock-service", "health", "watchlist"],
)

PythonOperator(
    task_id="ping_stock_service",
    python_callable=ping_stock_service,
    dag=dag,
)
