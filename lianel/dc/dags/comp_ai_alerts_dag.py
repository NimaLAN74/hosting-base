"""
Comp-AI Alerts DAG (G7).

Fetches gaps (controls with no evidence) and control tests; reports failed tests
and open gaps. Logs a summary; optionally sends to Slack if SLACK_WEBHOOK_URL
(Airflow Variable or env) is set.

Requires Airflow Variables: COMP_AI_BASE_URL, COMP_AI_TOKEN.
Optional: SLACK_WEBHOOK_URL for Slack notifications.
Optional: COMP_AI_APP_URL (e.g. https://www.lianel.se) for links in Slack.
See COMP-AI-AIRFLOW-RUNNER-DESIGN.md.
"""
from __future__ import annotations

import logging
import os
import time
from datetime import datetime, timedelta

import requests

from airflow import DAG
from airflow.providers.standard.operators.python import PythonOperator

# Add utils to path for comp_ai_client
import sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'utils'))
from comp_ai_client import get_gaps, get_tests

log = logging.getLogger(__name__)


def _get_variable(key: str, default: str | None = None) -> str | None:
    try:
        from airflow.sdk import Variable
        return Variable.get(key, default=default)
    except Exception:
        return default


def _get_app_url() -> str:
    """Base URL for 'View controls' link in Slack (optional)."""
    return (
        os.environ.get("COMP_AI_APP_URL")
        or _get_variable("COMP_AI_APP_URL")
        or "https://www.lianel.se"
    ).rstrip("/")

default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'start_date': datetime(2026, 1, 1),
    'email_on_failure': False,
    'email_on_retry': False,
    'retries': 1,
    'retry_delay': timedelta(minutes=2),
    'execution_timeout': timedelta(minutes=10),
}

dag = DAG(
    'comp_ai_alerts',
    default_args=default_args,
    description='Comp-AI alerts: gaps and failed control tests (log + optional Slack)',
    schedule='0 7 * * *',  # Daily at 07:00 UTC (after control_tests at 06:00)
    catchup=False,
    tags=['comp-ai', 'compliance', 'alerts', 'g7', 'scheduled'],
    max_active_runs=1,
)


def _get_slack_webhook_url() -> str | None:
    url = os.environ.get('SLACK_WEBHOOK_URL')
    if url:
        return url.strip()
    try:
        from airflow.sdk import Variable
        url = Variable.get('SLACK_WEBHOOK_URL', default=None)
        return url.strip() if url else None
    except Exception:
        return None


def _fetch_with_retry(get_fn, name: str, max_attempts: int = 2):
    """Call get_fn(); on failure retry once after 5s."""
    for attempt in range(max_attempts):
        try:
            return get_fn()
        except Exception as e:
            log.warning("%s attempt %s failed: %s", name, attempt + 1, e)
            if attempt + 1 < max_attempts:
                time.sleep(5)
            else:
                log.exception("%s failed after %s attempts", name, max_attempts)
                raise
    return None


def run_alerts(**context):
    """Fetch gaps and tests; build summary; log and optionally send to Slack."""
    gaps = []
    failed_tests = []
    error_msg = None
    try:
        gaps = _fetch_with_retry(get_gaps, "get_gaps")
    except Exception as e:
        error_msg = str(e)
    try:
        tests = _fetch_with_retry(get_tests, "get_tests")
        failed_tests = [t for t in (tests or []) if (t.get('last_result') or '').lower() == 'fail']
    except Exception as e:
        if not error_msg:
            error_msg = str(e)

    gap_count = len(gaps) if isinstance(gaps, list) else 0
    fail_count = len(failed_tests)
    has_issues = gap_count > 0 or fail_count > 0 or error_msg

    # Build human-readable summary
    lines = [
        "Comp-AI Alerts",
        f"Gaps (controls with no evidence): {gap_count}",
        f"Failed control tests: {fail_count}",
    ]
    if error_msg:
        lines.append(f"API error: {error_msg}")
    if gap_count > 0:
        for c in (gaps or [])[:10]:
            name = c.get('name') or c.get('requirement_id') or f"id={c.get('id')}"
            lines.append(f"  - Gap: {name}")
        if gap_count > 10:
            lines.append(f"  ... and {gap_count - 10} more")
    if fail_count > 0:
        for t in failed_tests[:10]:
            lines.append(f"  - Failed test: {t.get('name')} (control_id={t.get('control_id')})")
        if fail_count > 10:
            lines.append(f"  ... and {fail_count - 10} more")

    summary = "\n".join(lines)
    log.info(summary)
    context['ti'].xcom_push(key='summary', value=summary)
    context['ti'].xcom_push(key='gap_count', value=gap_count)
    context['ti'].xcom_push(key='failed_test_count', value=fail_count)
    context['ti'].xcom_push(key='has_issues', value=has_issues)

    # Optional: send to Slack (only if there are issues or we always report)
    webhook = _get_slack_webhook_url()
    controls_url = f"{_get_app_url()}/comp-ai/controls"
    if webhook:
        if has_issues:
            payload = {
                "text": summary,
                "blocks": [
                    {"type": "section", "text": {"type": "mrkdwn", "text": f"*Comp-AI Alerts*\n{summary}"}},
                    {"type": "section", "text": {"type": "mrkdwn", "text": f"<{controls_url}|View controls>"}},
                ],
            }
        else:
            payload = {
                "text": "Comp-AI: No gaps, no failed tests.",
                "blocks": [
                    {"type": "section", "text": {"type": "mrkdwn", "text": "Comp-AI: No gaps, no failed tests."}},
                    {"type": "section", "text": {"type": "mrkdwn", "text": f"<{controls_url}|View controls>"}},
                ],
            }
        try:
            r = requests.post(webhook, json=payload, timeout=10)
            r.raise_for_status()
            log.info("Slack notification sent")
        except Exception as e:
            log.warning("Slack notification failed: %s", e)

    return {'gap_count': gap_count, 'failed_test_count': fail_count, 'has_issues': has_issues}


alerts_task = PythonOperator(
    task_id='check_gaps_and_failed_tests',
    python_callable=run_alerts,
    dag=dag,
)
