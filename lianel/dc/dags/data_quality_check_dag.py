"""
Data Quality Check DAG

This DAG runs automated data quality validation checks on all ingested data:
- Completeness checks (missing values, gaps)
- Validity checks (constraints, ranges)
- Consistency checks (cross-table validation)
- Anomaly detection (outliers, sudden changes)
- Quality reporting

Schedule: Daily (02:00 UTC)
"""
from datetime import datetime, timedelta
from airflow import DAG
from airflow.operators.python import PythonOperator
from airflow.providers.postgres.hooks.postgres import PostgresHook
from airflow.exceptions import AirflowException
from typing import Dict, List, Any
import json

default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'start_date': datetime(2026, 1, 1),
    'email_on_failure': False,
    'email_on_retry': False,
    'retries': 1,
    'retry_delay': timedelta(minutes=5),
    'execution_timeout': timedelta(hours=1),
}

dag = DAG(
    'data_quality_check',
    default_args=default_args,
    description='Run automated data quality validation checks',
    schedule='0 2 * * *',  # Daily at 02:00 UTC
    catchup=False,
    tags=['data-quality', 'validation', 'monitoring'],
    max_active_runs=1,
)

# Quality thresholds
QUALITY_THRESHOLDS = {
    'max_missing_pct': 5.0,  # Max 5% missing values
    'max_negative_pct': 0.0,  # No negative values allowed
    'max_outlier_pct': 2.0,  # Max 2% outliers
    'max_yoy_change_pct': 50.0,  # Max 50% year-over-year change
}


def check_completeness(**context):
    """Check data completeness (missing values, gaps)"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    issues = []
    
    # Check 1: Missing values in fact_energy_annual
    sql = """
        SELECT 
            COUNT(*) FILTER (WHERE country_code IS NULL) as null_country,
            COUNT(*) FILTER (WHERE year IS NULL) as null_year,
            COUNT(*) FILTER (WHERE value_gwh IS NULL) as null_value,
            COUNT(*) FILTER (WHERE product_code IS NULL) as null_product,
            COUNT(*) FILTER (WHERE flow_code IS NULL) as null_flow,
            COUNT(*) as total_records
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
    """
    result = db_hook.get_first(sql)
    
    if result:
        total = result[5] if result[5] > 0 else 1
        null_country_pct = (result[0] / total) * 100
        null_year_pct = (result[1] / total) * 100
        null_value_pct = (result[2] / total) * 100
        null_product_pct = (result[3] / total) * 100
        null_flow_pct = (result[4] / total) * 100
        
        if null_country_pct > QUALITY_THRESHOLDS['max_missing_pct']:
            issues.append(f"⚠️  {null_country_pct:.2f}% missing country_code (threshold: {QUALITY_THRESHOLDS['max_missing_pct']}%)")
        if null_year_pct > QUALITY_THRESHOLDS['max_missing_pct']:
            issues.append(f"⚠️  {null_year_pct:.2f}% missing year (threshold: {QUALITY_THRESHOLDS['max_missing_pct']}%)")
        if null_value_pct > QUALITY_THRESHOLDS['max_missing_pct']:
            issues.append(f"⚠️  {null_value_pct:.2f}% missing value_gwh (threshold: {QUALITY_THRESHOLDS['max_missing_pct']}%)")
        if null_product_pct > QUALITY_THRESHOLDS['max_missing_pct']:
            issues.append(f"⚠️  {null_product_pct:.2f}% missing product_code (threshold: {QUALITY_THRESHOLDS['max_missing_pct']}%)")
        if null_flow_pct > QUALITY_THRESHOLDS['max_missing_pct']:
            issues.append(f"⚠️  {null_flow_pct:.2f}% missing flow_code (threshold: {QUALITY_THRESHOLDS['max_missing_pct']}%)")
        
        if not issues:
            print(f"✅ Completeness check passed: {total} records, all required fields present")
        else:
            for issue in issues:
                print(issue)
    
    # Check 2: Missing countries/years
    sql = """
        SELECT 
            COUNT(DISTINCT country_code) as countries,
            COUNT(DISTINCT year) as years,
            MIN(year) as earliest_year,
            MAX(year) as latest_year
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
    """
    result = db_hook.get_first(sql)
    
    if result:
        print(f"📊 Coverage: {result[0]} countries, {result[1]} years ({result[2]} - {result[3]})")
    
    context['ti'].xcom_push(key='completeness_issues', value=issues)
    return {'issues': issues, 'total_records': total if result else 0}


def check_validity(**context):
    """Check data validity (constraints, ranges)"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    issues = []
    
    # Check 1: Negative values
    sql = """
        SELECT COUNT(*) as negative_count
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
          AND value_gwh < 0
    """
    result = db_hook.get_first(sql)
    negative_count = result[0] if result else 0
    
    if negative_count > 0:
        issues.append(f"❌ Found {negative_count} records with negative values")
    else:
        print("✅ No negative values found")
    
    # Check 2: Invalid year range
    sql = """
        SELECT COUNT(*) as invalid_year_count
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
          AND (year < 1960 OR year > 2100)
    """
    result = db_hook.get_first(sql)
    invalid_year = result[0] if result else 0
    
    if invalid_year > 0:
        issues.append(f"❌ Found {invalid_year} records with invalid year")
    else:
        print("✅ All years in valid range (1960-2100)")
    
    # Check 3: Foreign key integrity
    sql = """
        SELECT COUNT(*) as fk_violations
        FROM fact_energy_annual f
        LEFT JOIN dim_country c ON f.country_code = c.country_code
        WHERE f.source_system = 'eurostat'
          AND f.country_code IS NOT NULL
          AND c.country_code IS NULL
    """
    result = db_hook.get_first(sql)
    fk_violations = result[0] if result else 0
    
    if fk_violations > 0:
        issues.append(f"❌ Found {fk_violations} foreign key violations (invalid country_code)")
    else:
        print("✅ Foreign key integrity maintained")
    
    # Check 4: Unit consistency
    sql = """
        SELECT COUNT(DISTINCT unit) as unit_types
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
          AND harmonisation_version IS NOT NULL
    """
    result = db_hook.get_first(sql)
    unit_types = result[0] if result else 0
    
    if unit_types > 1:
        issues.append(f"⚠️  Found {unit_types} different unit types after harmonization (expected: 1)")
    else:
        print("✅ Unit consistency: All harmonized data in GWh")
    
    context['ti'].xcom_push(key='validity_issues', value=issues)
    return {'issues': issues}


def check_consistency(**context):
    """Check data consistency (cross-table validation)"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    issues = []
    
    # Check 1: Year-over-year changes (detect sudden jumps)
    sql = """
        WITH yearly_totals AS (
            SELECT 
                country_code,
                year,
                SUM(value_gwh) as total_gwh
            FROM fact_energy_annual
            WHERE source_system = 'eurostat'
              AND value_gwh IS NOT NULL
            GROUP BY country_code, year
        ),
        yoy_changes AS (
            SELECT 
                y1.country_code,
                y1.year as current_year,
                y1.total_gwh as current_total,
                y2.total_gwh as prev_total,
                CASE 
                    WHEN y2.total_gwh > 0 THEN ABS((y1.total_gwh - y2.total_gwh) / y2.total_gwh * 100)
                    ELSE NULL
                END as change_pct
            FROM yearly_totals y1
            LEFT JOIN yearly_totals y2 ON y1.country_code = y2.country_code 
                AND y1.year = y2.year + 1
            WHERE y2.total_gwh IS NOT NULL
        )
        SELECT COUNT(*) as large_changes
        FROM yoy_changes
        WHERE change_pct > %s
    """
    result = db_hook.get_first(sql, parameters=(QUALITY_THRESHOLDS['max_yoy_change_pct'],))
    large_changes = result[0] if result else 0
    
    if large_changes > 0:
        issues.append(f"⚠️  Found {large_changes} country-year pairs with >{QUALITY_THRESHOLDS['max_yoy_change_pct']}% YoY change")
    else:
        print(f"✅ Year-over-year changes within threshold ({QUALITY_THRESHOLDS['max_yoy_change_pct']}%)")
    
    # Check 2: Data coverage by table
    sql = """
        SELECT 
            source_table,
            COUNT(*) as records,
            COUNT(DISTINCT country_code) as countries,
            COUNT(DISTINCT year) as years,
            MIN(year) as earliest,
            MAX(year) as latest
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
        GROUP BY source_table
        ORDER BY source_table
    """
    results = db_hook.get_records(sql)
    
    if results:
        print("📊 Data coverage by table:")
        for row in results:
            print(f"   {row[0]}: {row[1]} records, {row[2]} countries, {row[3]} years ({row[4]}-{row[5]})")
    
    context['ti'].xcom_push(key='consistency_issues', value=issues)
    return {'issues': issues}


def check_anomalies(**context):
    """Detect anomalies (outliers, sudden changes)"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    issues = []
    
    # Check 1: Statistical outliers (values > 3 standard deviations)
    sql = """
        WITH stats AS (
            SELECT 
                AVG(value_gwh) as mean_val,
                STDDEV(value_gwh) as stddev_val
            FROM fact_energy_annual
            WHERE source_system = 'eurostat'
              AND value_gwh IS NOT NULL
              AND value_gwh > 0
        )
        SELECT COUNT(*) as outliers
        FROM fact_energy_annual, stats
        WHERE source_system = 'eurostat'
          AND value_gwh IS NOT NULL
          AND ABS(value_gwh - stats.mean_val) > 3 * stats.stddev_val
    """
    result = db_hook.get_first(sql)
    outliers = result[0] if result else 0
    
    if outliers > 0:
        total_sql = "SELECT COUNT(*) FROM fact_energy_annual WHERE source_system = 'eurostat' AND value_gwh IS NOT NULL"
        total_result = db_hook.get_first(total_sql)
        total = total_result[0] if total_result else 1
        outlier_pct = (outliers / total) * 100
        
        if outlier_pct > QUALITY_THRESHOLDS['max_outlier_pct']:
            issues.append(f"⚠️  Found {outliers} outliers ({outlier_pct:.2f}%) - exceeds threshold")
        else:
            print(f"✅ Outliers within threshold: {outliers} ({outlier_pct:.2f}%)")
    else:
        print("✅ No statistical outliers detected")
    
    # Check 2: Zero values (might indicate missing data)
    sql = """
        SELECT COUNT(*) as zero_count
        FROM fact_energy_annual
        WHERE source_system = 'eurostat'
          AND value_gwh = 0
    """
    result = db_hook.get_first(sql)
    zero_count = result[0] if result else 0
    
    if zero_count > 0:
        print(f"ℹ️  Found {zero_count} records with zero values (may indicate missing data)")
    
    context['ti'].xcom_push(key='anomaly_issues', value=issues)
    return {'issues': issues, 'outliers': outliers, 'zeros': zero_count}


def generate_quality_report(**context):
    """Generate comprehensive quality report and store in meta_data_quality"""
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    ti = context['ti']
    
    # Collect all issues
    completeness_issues = ti.xcom_pull(task_ids='check_completeness', key='completeness_issues') or []
    validity_issues = ti.xcom_pull(task_ids='check_validity', key='validity_issues') or []
    consistency_issues = ti.xcom_pull(task_ids='check_consistency', key='consistency_issues') or []
    anomaly_issues = ti.xcom_pull(task_ids='check_anomalies', key='anomaly_issues') or []
    
    all_issues = completeness_issues + validity_issues + consistency_issues + anomaly_issues
    
    # Calculate quality score (0-100)
    total_checks = 10  # Approximate number of checks
    passed_checks = total_checks - len(all_issues)
    quality_score = (passed_checks / total_checks) * 100 if total_checks > 0 else 100
    
    # Determine overall status (must match DB constraint: 'pass', 'warn', 'fail')
    critical_issues = [i for i in all_issues if '❌' in i]
    if critical_issues:
        status = 'fail'
    elif all_issues:
        status = 'warn'
    else:
        status = 'pass'
    
    # Store in meta_data_quality (matching actual schema)
    sql = """
        INSERT INTO meta_data_quality 
        (table_name, quality_check, result, details)
        VALUES (%s, %s, %s, %s)
    """
    
    # Store overall report
    overall_details = {
        'quality_score': quality_score,
        'total_issues': len(all_issues),
        'critical_issues': len(critical_issues),
        'warnings': len(all_issues) - len(critical_issues),
        'issues': all_issues
    }
    db_hook.run(sql, parameters=(
        'fact_energy_annual',
        'comprehensive_quality_check',
        status,
        json.dumps(overall_details)
    ))
    
    # Store individual dimension reports
    for dimension, issues in [
        ('completeness', completeness_issues),
        ('validity', validity_issues),
        ('consistency', consistency_issues),
        ('accuracy', anomaly_issues)
    ]:
        dim_status = 'fail' if issues else 'pass'
        dim_details = {
            'issues_count': len(issues),
            'issues': issues
        }
        db_hook.run(sql, parameters=(
            'fact_energy_annual',
            f'{dimension}_check',
            dim_status,
            json.dumps(dim_details)
        ))
    
    # Print summary
    print(f"\n{'='*60}")
    print(f"📊 DATA QUALITY REPORT")
    print(f"{'='*60}")
    print(f"Overall Status: {status.upper()}")
    print(f"Quality Score: {quality_score:.1f}/100")
    print(f"Total Issues: {len(all_issues)}")
    print(f"  - Critical: {len(critical_issues)}")
    print(f"  - Warnings: {len(all_issues) - len(critical_issues)}")
    print(f"\nIssues by Dimension:")
    print(f"  - Completeness: {len(completeness_issues)}")
    print(f"  - Validity: {len(validity_issues)}")
    print(f"  - Consistency: {len(consistency_issues)}")
    print(f"  - Accuracy: {len(anomaly_issues)}")
    print(f"{'='*60}\n")
    
    if critical_issues:
        print("❌ CRITICAL ISSUES FOUND:")
        for issue in critical_issues:
            print(f"   {issue}")
    
    return {
        'status': status,
        'quality_score': quality_score,
        'total_issues': len(all_issues),
        'critical_issues': len(critical_issues)
    }


# DAG Tasks
start_task = PythonOperator(
    task_id='start',
    python_callable=lambda: print("🔍 Starting data quality checks"),
    dag=dag,
)

check_completeness_task = PythonOperator(
    task_id='check_completeness',
    python_callable=check_completeness,
    dag=dag,
)

check_validity_task = PythonOperator(
    task_id='check_validity',
    python_callable=check_validity,
    dag=dag,
)

check_consistency_task = PythonOperator(
    task_id='check_consistency',
    python_callable=check_consistency,
    dag=dag,
)

check_anomalies_task = PythonOperator(
    task_id='check_anomalies',
    python_callable=check_anomalies,
    dag=dag,
)

generate_report_task = PythonOperator(
    task_id='generate_quality_report',
    python_callable=generate_quality_report,
    dag=dag,
)

end_task = PythonOperator(
    task_id='end',
    python_callable=lambda: print("✅ Data quality check complete"),
    dag=dag,
)

# Task dependencies
start_task >> [check_completeness_task, check_validity_task, check_consistency_task, check_anomalies_task]
[check_completeness_task, check_validity_task, check_consistency_task, check_anomalies_task] >> generate_report_task
generate_report_task >> end_task

