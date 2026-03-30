"""
Stock Service – Quote Ingestion DAG (EU markets MVP)

Warms the backend quote cache on a schedule by calling GET /api/v1/quotes with a
configurable symbol list. The backend fetches from its providers (Yahoo/Stooq/Alpha Vantage)
and caches results; the alerts DAG runs after this to evaluate price alerts.

Symbol list: set Airflow Variable "stock_monitoring_ingest_symbols" (comma-separated),
e.g. "ASML.AS,SAP.DE,VOLV-B.ST,SHEL.L,AAPL". Defaults to a small EU set if unset.
"""

import json
import os
from datetime import datetime, timedelta
from urllib import parse, request, error

from airflow import DAG
from airflow.models import Variable
from airflow.exceptions import AirflowSkipException
from airflow.operators.python import PythonOperator

# Default EU symbols when Variable is not set
DEFAULT_INGEST_SYMBOLS = "ASML.AS,SAP.DE,VOLV-B.ST,SHEL.L"


def ingest_eu_quotes(**context):
    """Fetch quotes for symbols via GET /internal/quotes/fetch?symbols=... (no auth). Backend persists to intraday; roll DAG aggregates to price_history_daily for 7-day chart."""
    base_url = os.getenv("STOCK_MONITORING_INTERNAL_URL", "http://lianel-stock-service:3003")
    symbols_str = Variable.get("stock_monitoring_ingest_symbols", default_var=DEFAULT_INGEST_SYMBOLS)
    symbols_str = (symbols_str or "").strip() or DEFAULT_INGEST_SYMBOLS
    symbols_param = parse.quote(symbols_str)
    url = f"{base_url.rstrip('/')}/internal/quotes/fetch?symbols={symbols_param}"
    req = request.Request(url=url, method="GET")
    try:
        with request.urlopen(req, timeout=60) as response:  # nosec B310 - trusted internal endpoint
            status = response.status
            body = response.read().decode("utf-8")
    except error.HTTPError as e:
        body = e.read().decode("utf-8", errors="replace") if hasattr(e, "read") else str(e)
        if e.code >= 500:
            raise AirflowSkipException(f"Stock service transient HTTP {e.code}: {body[:500]}")
        raise RuntimeError(f"Quotes fetch failed: HTTP {e.code} - {body[:500]}")
    except error.URLError as e:
        raise AirflowSkipException(f"Stock service unavailable (network): {e}")
    if status < 200 or status >= 300:
        if status >= 500:
            raise AirflowSkipException(f"Stock service transient HTTP {status}: {body[:500]}")
        raise RuntimeError(f"Quotes fetch failed: HTTP {status} - {body[:500]}")
    try:
        data = json.loads(body)
        count = data.get("count", 0)
        print(f"Ingest complete: {count} quotes fetched and persisted to intraday (symbols: {len(symbols_str.split(','))})")
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
    tags=["stock-service", "eu-markets", "quotes", "mvp"],
)

PythonOperator(
    task_id="ingest_eu_quotes",
    python_callable=ingest_eu_quotes,
    dag=dag,
)
