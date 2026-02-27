"""
Stock Monitoring – Symbol catalog sync DAG

Calls the backend internal endpoint POST /internal/symbols/refresh?provider=finnhub
to fetch symbols from Finnhub (US, LON, AS, DE) and upsert them into stock_monitoring.symbols.
The backend then serves GET /api/v1/symbols from the DB, so symbol lists don't change every request.
Run daily or a few times per week (symbols change rarely).
"""

import os
from datetime import datetime, timedelta
from urllib import parse, request

from airflow import DAG
from airflow.operators.python import PythonOperator


def sync_symbols(**context):
    """POST /internal/symbols/refresh?provider=finnhub to refresh symbol catalog in DB."""
    base_url = os.getenv("STOCK_MONITORING_INTERNAL_URL", "http://lianel-stock-monitoring-service:3003")
    url = f"{base_url.rstrip('/')}/internal/symbols/refresh?provider=finnhub"
    req = request.Request(url=url, method="POST", data=b"")
    req.add_header("Content-Type", "application/json")
    with request.urlopen(req, timeout=300) as response:  # nosec B310 - trusted internal endpoint
        status = response.status
        body = response.read().decode("utf-8")
    if status < 200 or status >= 300:
        raise RuntimeError(f"Symbol refresh failed: HTTP {status} - {body[:500]}")
    try:
        import json
        data = json.loads(body)
        synced = data.get("synced", 0)
        print(f"Symbol sync complete: {synced} symbols upserted for provider={data.get('provider', 'finnhub')}")
    except Exception:
        print(f"Symbol sync complete: HTTP {status}")


default_args = {
    "owner": "lianel",
    "depends_on_past": False,
    "start_date": datetime(2026, 1, 1),
    "email_on_failure": False,
    "retries": 2,
    "retry_delay": timedelta(minutes=5),
}

dag = DAG(
    "stock_monitoring_symbol_sync",
    default_args=default_args,
    description="Refresh symbol catalog from Finnhub into DB (backend serves list from DB)",
    schedule="0 3 * * 1,4",  # 03:00 UTC on Monday and Thursday
    catchup=False,
    tags=["stock-monitoring", "symbols", "finnhub", "mvp"],
)

PythonOperator(
    task_id="sync_symbols",
    python_callable=sync_symbols,
    dag=dag,
)
