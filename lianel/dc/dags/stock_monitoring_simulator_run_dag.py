"""
Stock Service - Simulator DAG

Keeps a **single active** live (or replay) simulator run when none exists, on a
short interval. Default payload targets **~6+ months** of wall-clock steps at
60s cadence (`max_cycles` ≈ 500k). Change policy via env without editing code:

- `SIM_REPLAY_RESTART_POLICY=always` (default): when no run is active, start one
  (including after the previous run finished with MAX_CYCLES_REACHED or MANUAL_STOP).
- `SIM_REPLAY_RESTART_POLICY=bankrupt_only`: only auto-start when the latest
  completed run ended in **BANKRUPT** (or there are no runs). Use this if you want
  one long campaign per manual reset instead of chaining runs after each completion.

Research / 126d training replay (IBKR daily bars, strict overlap):

- `SIM_REPLAY_RESEARCH_MODE=true`: sets `live_market_data=false`, `replay_delay_ms=0` (override with
  `SIM_REPLAY_DELAY_MS`), and defaults `SIM_REPLAY_REQUIRE_FULL_HORIZON=true` so the run **fails**
  if aligned history is shorter than `SIM_REPLAY_DAYS` (no silent shrink).
"""

import json
import os
import inspect
from datetime import datetime, timedelta
from urllib import request, parse

from airflow import DAG
try:
    # Airflow 2.4+ location.
    from airflow.operators.python import PythonOperator
except Exception:  # pragma: no cover - compatibility for older Airflow runtimes
    # Legacy location used by older Airflow installs.
    from airflow.operators.python_operator import PythonOperator


def run_simulator_replay(**context):
    base_url = (
        os.getenv("STOCK_SERVICE_INTERNAL_URL")
        or os.getenv("STOCK_MONITORING_INTERNAL_URL")
        or "http://lianel-stock-service:3003"
    )
    days = int(os.getenv("SIM_REPLAY_DAYS", "126"))
    limit = int(os.getenv("SIM_REPLAY_RUN_LIST_LIMIT", "20"))
    runs_url = f"{base_url.rstrip('/')}/api/v1/stock-service/sim/runs?{parse.urlencode({'limit': max(5, min(limit, 100))})}"
    with request.urlopen(runs_url, timeout=30) as response:  # nosec B310 - trusted internal endpoint
        runs_raw = response.read().decode("utf-8")
    runs_data = json.loads(runs_raw) if runs_raw else {}
    runs = runs_data.get("runs", []) if isinstance(runs_data, dict) else []
    latest = runs[0] if runs else None
    latest_status = str((latest or {}).get("status", "")).lower()
    latest_stop_reason = str((latest or {}).get("stop_reason", "")).upper()
    active_statuses = {"queued", "running", "paused"}
    has_active = any(str((r or {}).get("status", "")).lower() in active_statuses for r in runs if isinstance(r, dict))

    # Continuous mode: keep exactly one active run. If a run is already active, skip.
    if has_active:
        print("Simulator run already active; skip trigger.")
        return

    restart_policy = os.getenv("SIM_REPLAY_RESTART_POLICY", "always").strip().lower()
    if restart_policy == "bankrupt_only" and latest and latest_status == "completed":
        if latest_stop_reason != "BANKRUPT":
            print(
                f"SIM_REPLAY_RESTART_POLICY=bankrupt_only: latest run completed with "
                f"stop_reason={latest_stop_reason or 'n/a'}; skip auto-start until BANKRUPT or manual purge."
            )
            return

    # Start a new run when no active run exists (policy allows).
    if latest and latest_status == "completed" and latest_stop_reason == "BANKRUPT":
        print("Latest run ended BANKRUPT; starting replacement run.")
    elif latest:
        print(f"No active run. Latest status={latest_status or 'unknown'} stop_reason={latest_stop_reason or 'n/a'}. Starting new run.")

    research_mode = os.getenv("SIM_REPLAY_RESEARCH_MODE", "").strip().lower() in ("1", "true", "yes")
    if research_mode:
        live_md = False
        replay_delay = int(os.getenv("SIM_REPLAY_DELAY_MS", "0"))
        require_full = os.getenv("SIM_REPLAY_REQUIRE_FULL_HORIZON", "true").lower() in ("1", "true", "yes")
    else:
        live_md = os.getenv("SIM_REPLAY_LIVE_MARKET_DATA", "true").lower() in ("1", "true", "yes")
        replay_delay = int(os.getenv("SIM_REPLAY_DELAY_MS", "60000"))
        require_full = os.getenv("SIM_REPLAY_REQUIRE_FULL_HORIZON", "false").lower() in ("1", "true", "yes")

    base_payload = {
        "top": int(os.getenv("SIM_REPLAY_TOP", "16")),
        "quantile": float(os.getenv("SIM_REPLAY_QUANTILE", "0.2")),
        "short_enabled": os.getenv("SIM_REPLAY_SHORT_ENABLED", "true").lower() in ("1", "true", "yes"),
        "initial_capital_usd": float(os.getenv("SIM_REPLAY_INITIAL_CAPITAL_USD", "100")),
        "reinvest_profit": os.getenv("SIM_REPLAY_REINVEST", "true").lower() in ("1", "true", "yes"),
        "live_market_data": live_md,
        "replay_delay_ms": replay_delay,
        "readiness_min_days": int(os.getenv("SIM_REPLAY_READINESS_MIN_DAYS", "126")),
        "replay_require_full_horizon": require_full,
        # ~500k cycles at 60s step ≈ 347 days wall-clock (upper bound; live mode also skips closed markets).
        "max_cycles": int(os.getenv("SIM_REPLAY_MAX_CYCLES", "500000")),
    }
    target_days = max(7, min(days, 365))
    candidate_days = [target_days, 126, 90, 60, 30, 14, 7]
    # Preserve order and uniqueness.
    unique_days = []
    seen = set()
    for d in candidate_days:
        if d not in seen:
            unique_days.append(d)
            seen.add(d)
    url = f"{base_url.rstrip('/')}/api/v1/stock-service/sim/runs"
    last_error = None
    for d in unique_days:
        payload = {**base_payload, "days": d}
        body = json.dumps(payload).encode("utf-8")
        req = request.Request(
            url=url,
            method="POST",
            data=body,
            headers={"Content-Type": "application/json", "Accept": "application/json"},
        )
        try:
            with request.urlopen(req, timeout=90) as response:  # nosec B310 - trusted internal endpoint
                status = response.status
                raw = response.read().decode("utf-8")
            if status < 200 or status >= 300:
                last_error = f"HTTP {status} - {raw[:500]}"
                continue
            data = json.loads(raw)
            run_id = data.get("run_id", "<unknown>")
            print(f"Simulator run started: {run_id} (days={d})")
            return
        except Exception as e:
            last_error = str(e)
            continue
    raise RuntimeError(f"Simulator run trigger failed for all day windows {unique_days}: {last_error}")


default_args = {
    "owner": "lianel",
    "depends_on_past": False,
    "start_date": datetime(2026, 1, 1),
    "email_on_failure": False,
    "retries": 1,
    "retry_delay": timedelta(minutes=5),
}

DAG_ID = "stock_monitoring_simulator_replay"

_dag_kwargs = dict(
    dag_id=DAG_ID,
    default_args=default_args,
    description="Trigger stock replay simulator to find model/process/data gaps",
    catchup=False,
    tags=["stock-service", "simulator", "bias-detection"],
)
_dag_schedule = "*/30 * * * *"  # Keep one active run; restart quickly after completion/bankrupt.
_dag_params = inspect.signature(DAG).parameters
if "schedule" in _dag_params:
    dag = DAG(**_dag_kwargs, schedule=_dag_schedule)
else:
    dag = DAG(**_dag_kwargs, schedule_interval=_dag_schedule)

run_task = PythonOperator(
    task_id="run_simulator_replay",
    python_callable=run_simulator_replay,
    dag=dag,
)

# Some runtimes are strict about symbol exposure while loading bundles.
globals()["stock_monitoring_simulator_replay"] = dag
# Also expose a symbol whose name matches dag_id exactly.
stock_monitoring_simulator_replay = dag

