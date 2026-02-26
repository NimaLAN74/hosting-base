"""
Stock Monitoring – Quote Ingestion DAG (EU markets MVP)

Warms the backend quote cache on a schedule by calling GET /api/v1/quotes with a
configurable symbol list. The backend fetches from its providers (Yahoo/Stooq/Alpha Vantage)
and caches results; the alerts DAG runs after this to evaluate price alerts.

Symbol list: set Airflow Variable "stock_monitoring_ingest_symbols" (comma-separated),
e.g. "ASML.AS,SAP.DE,VOLV-B.ST,SHEL.L,AAPL". Defaults to a small EU set if unset.
"""

import json
import os
from datetime import datetime, timedelta
from urllib import parse, request

from airflow import DAG
from airflow.models import Variable
from airflow.operators.python import PythonOperator

# Default EU symbols when Variable is not set
DEFAULT_INGEST_SYMBOLS = "ASML.AS,SAP.DE,VOLV-B.ST,SHEL.L"


def ingest_eu_quotes(**context):
    """Warm backend quote cache by calling GET /api/v1/quotes?symbols=..."""
    base_url = os.getenv("STOCK_MONITORING_INTERNAL_URL", "http://lianel-stock-monitoring-service:3003")
    symbols_str = Variable.get("stock_monitoring_ingest_symbols", default=DEFAULT_INGEST_SYMBOLS)
    symbols_str = (symbols_str or "").strip() or DEFAULT_INGEST_SYMBOLS
    symbols_param = parse.quote(symbols_str)
    url = f"{base_url.rstrip('/')}/api/v1/quotes?symbols={symbols_param}"
    req = request.Request(url=url, method="GET")
    with request.urlopen(req, timeout=60) as response:  # nosec B310 - trusted internal endpoint
        status = response.status
        body = response.read().decode("utf-8")
    if status < 200 or status >= 300:
        raise RuntimeError(f"Quotes request failed: HTTP {status} - {body[:500]}")
    # Log summary (body can be large)
    try:
        data = json.loads(body)
        quotes = data.get("quotes", [])
        print(f"Ingest complete: {len(quotes)} quotes cached (symbols requested: {len(symbols_str.split(','))})")
    except Exception:
        print(f"Ingest complete: HTTP {status}")


default_args = {
    "owner": "lianel",
    "depends_on_past": False,
    "start_date": datetime(2026, 1, 1),
    "email_on_failure": False,
    "retries": 1,
    "retry_delay": timedelta(minutes=5),
}

dag = DAG(
    "stock_monitoring_ingest",
    default_args=default_args,
    description="Warm backend quote cache for EU symbols (schedule triggers fetch and cache)",
    schedule="0 8-17 * * 1-5",  # Hourly 08–17 UTC on weekdays (EU session)
    catchup=False,
    tags=["stock-monitoring", "eu-markets", "quotes", "mvp"],
)

PythonOperator(
    task_id="ingest_eu_quotes",
    python_callable=ingest_eu_quotes,
    dag=dag,
)
