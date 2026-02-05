"""
Comp-AI Organisation Document Scan DAG (Phase C – automated scanning).

Runs on a schedule (e.g. weekly) and calls POST /api/v1/scan/documents
to create evidence from a list of document URLs for the whole organisation.

Configuration: set Airflow Variable COMP_AI_SCAN_DOCUMENTS_CONFIG (JSON).
  - Single job: {"control_id": 2, "documents": [{"url": "https://..."}, {"url": "https://...", "type": "policy"}]}
  - Multiple jobs: [{"control_id": 2, "documents": [...]}, {"control_id": 3, "documents": [...]}]
If unset or empty, the DAG runs successfully but creates no evidence (no-op).

Requires: COMP_AI_BASE_URL, COMP_AI_TOKEN (or client credentials).
See COMP-AI-AIRFLOW-RUNNER-DESIGN.md.
"""
from __future__ import annotations

import json
import logging
import os
from datetime import datetime, timedelta

from airflow import DAG
from airflow.providers.standard.operators.python import PythonOperator

# Add utils to path for comp_ai_client
import sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'utils'))
from comp_ai_client import _get_config, post_scan_documents, post_analysis_gaps

log = logging.getLogger(__name__)


def _get_scan_config() -> list[dict] | None:
    """Read COMP_AI_SCAN_DOCUMENTS_CONFIG from Variable or env. Returns list of jobs or None."""
    raw = os.environ.get('COMP_AI_SCAN_DOCUMENTS_CONFIG')
    if not raw:
        try:
            from airflow.sdk import Variable
            raw = Variable.get('COMP_AI_SCAN_DOCUMENTS_CONFIG', default=None)
        except Exception:
            pass
    if not raw or not str(raw).strip():
        return None
    try:
        data = json.loads(raw)
        if isinstance(data, list):
            return data
        if isinstance(data, dict) and "control_id" in data and "documents" in data:
            return [data]
        return None
    except json.JSONDecodeError as e:
        log.warning("Invalid COMP_AI_SCAN_DOCUMENTS_CONFIG JSON: %s", e)
        return None


default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'start_date': datetime(2026, 1, 1),
    'email_on_failure': False,
    'email_on_retry': False,
    'retries': 1,
    'retry_delay': timedelta(minutes=2),
    'execution_timeout': timedelta(minutes=30),
}

dag = DAG(
    'comp_ai_scan_documents',
    default_args=default_args,
    description='Comp-AI organisation document scan: batch create evidence from URLs (Phase C)',
    schedule='0 8 * * 0',  # Weekly: Sunday 08:00 UTC
    catchup=False,
    tags=['comp-ai', 'compliance', 'scan', 'organisation', 'phase-c', 'scheduled'],
    max_active_runs=1,
)


def run_organisation_scan(**context):
    """
    Run document scan for the organisation.
    Reads COMP_AI_SCAN_DOCUMENTS_CONFIG; for each job calls POST /api/v1/scan/documents.
    """
    jobs = _get_scan_config()
    if not jobs:
        log.info("COMP_AI_SCAN_DOCUMENTS_CONFIG not set or empty; skipping scan (0 evidence created).")
        context['ti'].xcom_push(key='total_created', value=0)
        context['ti'].xcom_push(key='jobs_run', value=0)
        return {'total_created': 0, 'jobs_run': 0, 'results': []}

    base_url, token = _get_config()
    total_created = 0
    results = []
    for i, job in enumerate(jobs):
        control_id = job.get('control_id')
        documents = job.get('documents') or []
        if not documents or control_id is None:
            log.warning("Job %d: missing control_id or documents; skip", i + 1)
            continue
        try:
            out = post_scan_documents(base_url, token, int(control_id), documents)
            created = out.get('created', 0)
            total_created += created
            results.append({
                'control_id': control_id,
                'created': created,
                'evidence_ids': out.get('evidence_ids', []),
            })
            log.info("Job %d: control_id=%s created %d evidence item(s)", i + 1, control_id, created)
        except Exception as e:
            log.exception("Job %d (control_id=%s) failed: %s", i + 1, control_id, e)
            results.append({'control_id': control_id, 'error': str(e)})
            raise

    context['ti'].xcom_push(key='total_created', value=total_created)
    context['ti'].xcom_push(key='jobs_run', value=len(results))
    context['ti'].xcom_push(key='results', value=results)
    log.info("Organisation scan complete: %d job(s), %d evidence created", len(results), total_created)
    return {'total_created': total_created, 'jobs_run': len(results), 'results': results}


def run_gap_analysis_monitoring(**context):
    """
    Run AI gap/risk analysis and log summary (organisation monitoring).
    POST /api/v1/analysis/gaps; optional framework filter via Variable COMP_AI_GAP_ANALYSIS_FRAMEWORK.
    """
    base_url, token = _get_config()
    framework = os.environ.get('COMP_AI_GAP_ANALYSIS_FRAMEWORK')
    if not framework:
        try:
            from airflow.sdk import Variable
            framework = Variable.get('COMP_AI_GAP_ANALYSIS_FRAMEWORK', default=None)
        except Exception:
            pass
    try:
        out = post_analysis_gaps(base_url, token, framework)
        summary = out.get('summary', '') or 'No summary'
        model = out.get('model_used', '')
        log.info("Gap analysis (model=%s): %s", model, summary[:500] + ('...' if len(summary) > 500 else ''))
        context['ti'].xcom_push(key='gap_summary', value=summary)
        context['ti'].xcom_push(key='model_used', value=model)
        return {'summary_preview': summary[:500], 'model_used': model}
    except Exception as e:
        log.warning("Gap analysis failed (non-fatal): %s", e)
        context['ti'].xcom_push(key='gap_analysis_error', value=str(e))
        return {'error': str(e)}


scan_task = PythonOperator(
    task_id='run_organisation_document_scan',
    python_callable=run_organisation_scan,
    dag=dag,
)

gap_analysis_task = PythonOperator(
    task_id='run_gap_analysis_monitoring',
    python_callable=run_gap_analysis_monitoring,
    dag=dag,
)

# Run gap analysis after scan (monitoring: scan then analyse gaps)
scan_task >> gap_analysis_task
