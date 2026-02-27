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
    """POST /internal/symbols/refresh?provider=finnhub to refresh symbol catalog in DB.
    Backend batches DB upserts; allow up to 10 min for 4 exchanges."""
    import time
    base_url = os.getenv("STOCK_MONITORING_INTERNAL_URL", "http://lianel-stock-monitoring-service:3003")
    url = f"{base_url.rstrip('/')}/internal/symbols/refresh?provider=finnhub"
    req = request.Request(url=url, method="POST", data=b"")
    req.add_header("Content-Type", "application/json")
    last_err = None
    for attempt in range(3):
        try:
            with request.urlopen(req, timeout=600) as response:  # 10 min - nosec B310 trusted internal
                status = response.status
                body = response.read().decode("utf-8")
            if status < 200 or status >= 300:
                raise RuntimeError(f"Symbol refresh failed: HTTP {status} - {body[:500]}")
            import json
            data = json.loads(body)
            synced = data.get("synced", 0)
            print(f"Symbol sync complete: {synced} symbols upserted for provider={data.get('provider', 'finnhub')}")
            return
        except OSError as e:
            last_err = e
            if attempt < 2:
                time.sleep(30 * (attempt + 1))
    raise RuntimeError(f"Symbol refresh failed after retries: {last_err}")


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
