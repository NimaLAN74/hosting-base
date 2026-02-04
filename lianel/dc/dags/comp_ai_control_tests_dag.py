"""
Comp-AI Control Tests DAG (Airflow as runner).

Fetches all control tests from Comp-AI, runs each (stub or real check),
and records results via POST /api/v1/controls/:id/tests/:test_id/result.

Requires Airflow Variables: COMP_AI_BASE_URL, COMP_AI_TOKEN (or set in env).
See COMP-AI-AIRFLOW-RUNNER-DESIGN.md.
"""
from __future__ import annotations

import os
from datetime import datetime, timedelta

from airflow import DAG
from airflow.exceptions import AirflowException
from airflow.providers.standard.operators.python import PythonOperator

# Add utils to path for comp_ai_client
import sys
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'utils'))
from comp_ai_client import _get_config, get_tests, run_test_and_record

default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'start_date': datetime(2026, 1, 1),
    'email_on_failure': False,
    'email_on_retry': False,
    'retries': 2,
    'retry_delay': timedelta(minutes=2),
    'execution_timeout': timedelta(minutes=15),
}

dag = DAG(
    'comp_ai_control_tests',
    default_args=default_args,
    description='Run Comp-AI control tests and record results (Airflow as runner)',
    schedule='0 6 * * *',  # Daily at 06:00 UTC
    catchup=False,
    tags=['comp-ai', 'compliance', 'tests', 'scheduled'],
    max_active_runs=1,
)


def run_all_control_tests(**context):
    """Fetch tests from Comp-AI, run each (stub), post result back."""
    base_url, token = _get_config()
    tests = get_tests()
    if not tests:
        context['ti'].xcom_push(key='tests_run', value=0)
        context['ti'].xcom_push(key='message', value='No tests to run')
        return {'tests_run': 0, 'results': []}
    results = []
    for test in tests:
        try:
            updated = run_test_and_record(test, base_url, token)
            results.append({
                'test_id': test['id'],
                'control_id': test['control_id'],
                'name': test.get('name'),
                'result': updated.get('last_result', 'pass'),
            })
        except Exception as e:
            results.append({
                'test_id': test['id'],
                'control_id': test['control_id'],
                'name': test.get('name'),
                'error': str(e),
            })
            # Continue with other tests; one failure does not fail the whole DAG
    context['ti'].xcom_push(key='tests_run', value=len(results))
    context['ti'].xcom_push(key='results', value=results)
    return {'tests_run': len(results), 'results': results}


run_tests_task = PythonOperator(
    task_id='run_control_tests_and_record',
    python_callable=run_all_control_tests,
    dag=dag,
)
