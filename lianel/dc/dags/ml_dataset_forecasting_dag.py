"""
ML Forecasting Dataset DAG

Creates a forecasting-ready dataset from energy annual data.
Since we have annual data (not hourly), this DAG creates:
- Time-based features (year, year_index)
- Lagged values (previous 1-3 years)
- Rolling statistics (3-year, 5-year means)
- Year-over-year changes
- Trend indicators

Table: ml_dataset_forecasting_v1
"""

from datetime import datetime, timedelta
from airflow import DAG
from airflow.providers.postgres.hooks.postgres import PostgresHook
from airflow.operators.python import PythonOperator
import logging

# Default arguments
default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'email_on_failure': False,
    'email_on_retry': False,
    'retries': 1,
    'retry_delay': timedelta(minutes=5),
}

# DAG definition
dag = DAG(
    'ml_dataset_forecasting',
    default_args=default_args,
    description='Create ML forecasting dataset with time-based features',
    schedule='0 5 * * 0',  # Every Sunday at 05:00 UTC (after harmonization)
    start_date=datetime(2026, 1, 1),
    catchup=False,
    tags=['ml', 'dataset', 'forecasting', 'lianel'],
    max_active_runs=1,
)


def create_forecasting_dataset_table(**context):
    """
    Create ml_dataset_forecasting_v1 table if it doesn't exist.
    """
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    logging.info("Creating ml_dataset_forecasting_v1 table if it doesn't exist...")
    
    create_table_sql = """
    CREATE TABLE IF NOT EXISTS ml_dataset_forecasting_v1 (
        dataset_id BIGSERIAL PRIMARY KEY,
        region_id VARCHAR(10) NOT NULL,
        level_code INTEGER NOT NULL,
        cntr_code VARCHAR(2) NOT NULL,
        region_name VARCHAR(255),
        year INTEGER NOT NULL,
        
        -- Target variables (for forecasting)
        total_energy_gwh NUMERIC(15,3),
        renewable_energy_gwh NUMERIC(15,3),
        fossil_energy_gwh NUMERIC(15,3),
        
        -- Time-based features
        year_index INTEGER,  -- 0 for first year, 1 for second, etc.
        is_first_year BOOLEAN,
        is_last_year BOOLEAN,
        
        -- Lagged values (previous years)
        lag_1_year_total_energy_gwh NUMERIC(15,3),
        lag_2_year_total_energy_gwh NUMERIC(15,3),
        lag_3_year_total_energy_gwh NUMERIC(15,3),
        lag_1_year_renewable_gwh NUMERIC(15,3),
        lag_2_year_renewable_gwh NUMERIC(15,3),
        
        -- Year-over-year changes
        yoy_change_total_energy_pct NUMERIC(10,3),
        yoy_change_renewable_pct NUMERIC(10,3),
        yoy_change_absolute_gwh NUMERIC(15,3),
        
        -- Rolling statistics (moving averages)
        rolling_3y_mean_total_energy_gwh NUMERIC(15,3),
        rolling_5y_mean_total_energy_gwh NUMERIC(15,3),
        rolling_3y_mean_renewable_gwh NUMERIC(15,3),
        rolling_5y_mean_renewable_gwh NUMERIC(15,3),
        
        -- Trend indicators
        trend_3y_slope NUMERIC(10,3),  -- Linear trend over 3 years
        trend_5y_slope NUMERIC(10,3),  -- Linear trend over 5 years
        is_increasing_trend BOOLEAN,
        is_decreasing_trend BOOLEAN,
        
        -- Percentage features
        pct_renewable NUMERIC(5,2),
        pct_fossil NUMERIC(5,2),
        
        -- Spatial features (from dim_region)
        area_km2 DOUBLE PRECISION,
        energy_density_gwh_per_km2 NUMERIC(15,3),
        
        -- Metadata
        feature_count INTEGER,
        created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
        updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
        
        CONSTRAINT uk_forecasting_region_year UNIQUE (region_id, year),
        CONSTRAINT fk_forecasting_region FOREIGN KEY (region_id) 
            REFERENCES dim_region(region_id) ON DELETE RESTRICT
    );
    
    -- Create indexes
    CREATE INDEX IF NOT EXISTS idx_forecasting_region_year 
        ON ml_dataset_forecasting_v1(region_id, year);
    CREATE INDEX IF NOT EXISTS idx_forecasting_year 
        ON ml_dataset_forecasting_v1(year);
    CREATE INDEX IF NOT EXISTS idx_forecasting_cntr_code 
        ON ml_dataset_forecasting_v1(cntr_code);
    """
    
    db_hook.run(create_table_sql)
    logging.info("✅ Table ml_dataset_forecasting_v1 created/verified")
    
    return {'table_created': True}


def extract_forecasting_features(**context):
    """
    Extract time-based features from energy data.
    This creates the base dataset with lagged values and rolling statistics.
    """
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    logging.info("Extracting forecasting features from energy data...")
    
    # Extract features with lagged values and rolling statistics
    extract_sql = """
    WITH energy_by_region_year AS (
        SELECT 
            r.region_id,
            r.level_code,
            r.cntr_code,
            r.name_latn as region_name,
            e.year,
            SUM(e.value_gwh) as total_energy_gwh,
            SUM(CASE WHEN p.renewable_flag = TRUE THEN e.value_gwh ELSE 0 END) as renewable_energy_gwh,
            SUM(CASE WHEN p.fossil_flag = TRUE THEN e.value_gwh ELSE 0 END) as fossil_energy_gwh,
            COUNT(*) as feature_count
        FROM fact_energy_annual e
        INNER JOIN dim_region r ON e.country_code = r.cntr_code AND r.level_code = 0
        LEFT JOIN dim_energy_product p ON e.product_code = p.product_code
        WHERE e.value_gwh IS NOT NULL AND e.value_gwh > 0
        GROUP BY r.region_id, r.level_code, r.cntr_code, r.name_latn, e.year
    ),
    with_lags AS (
        SELECT 
            *,
            LAG(total_energy_gwh, 1) OVER (PARTITION BY region_id ORDER BY year) as lag_1_year_total_energy_gwh,
            LAG(total_energy_gwh, 2) OVER (PARTITION BY region_id ORDER BY year) as lag_2_year_total_energy_gwh,
            LAG(total_energy_gwh, 3) OVER (PARTITION BY region_id ORDER BY year) as lag_3_year_total_energy_gwh,
            LAG(renewable_energy_gwh, 1) OVER (PARTITION BY region_id ORDER BY year) as lag_1_year_renewable_gwh,
            LAG(renewable_energy_gwh, 2) OVER (PARTITION BY region_id ORDER BY year) as lag_2_year_renewable_gwh,
            year - MIN(year) OVER (PARTITION BY region_id) as year_index,
            year = MIN(year) OVER (PARTITION BY region_id) as is_first_year,
            year = MAX(year) OVER (PARTITION BY region_id) as is_last_year
        FROM energy_by_region_year
    ),
    with_rolling_stats AS (
        SELECT 
            *,
            AVG(total_energy_gwh) OVER (
                PARTITION BY region_id 
                ORDER BY year 
                ROWS BETWEEN 2 PRECEDING AND CURRENT ROW
            ) as rolling_3y_mean_total_energy_gwh,
            AVG(total_energy_gwh) OVER (
                PARTITION BY region_id 
                ORDER BY year 
                ROWS BETWEEN 4 PRECEDING AND CURRENT ROW
            ) as rolling_5y_mean_total_energy_gwh,
            AVG(renewable_energy_gwh) OVER (
                PARTITION BY region_id 
                ORDER BY year 
                ROWS BETWEEN 2 PRECEDING AND CURRENT ROW
            ) as rolling_3y_mean_renewable_gwh,
            AVG(renewable_energy_gwh) OVER (
                PARTITION BY region_id 
                ORDER BY year 
                ROWS BETWEEN 4 PRECEDING AND CURRENT ROW
            ) as rolling_5y_mean_renewable_gwh
        FROM with_lags
    ),
    with_yoy_changes AS (
        SELECT 
            *,
            CASE 
                WHEN lag_1_year_total_energy_gwh > 0 
                THEN ((total_energy_gwh - lag_1_year_total_energy_gwh) / lag_1_year_total_energy_gwh * 100)
                ELSE NULL 
            END as yoy_change_total_energy_pct,
            CASE 
                WHEN lag_1_year_renewable_gwh > 0 
                THEN ((renewable_energy_gwh - lag_1_year_renewable_gwh) / lag_1_year_renewable_gwh * 100)
                ELSE NULL 
            END as yoy_change_renewable_pct,
            (total_energy_gwh - lag_1_year_total_energy_gwh) as yoy_change_absolute_gwh
        FROM with_rolling_stats
    ),
    with_trends AS (
        SELECT 
            *,
            -- Calculate linear trend slope over 3 years
            CASE 
                WHEN year >= MIN(year) OVER (PARTITION BY region_id) + 2
                THEN (
                    (total_energy_gwh - LAG(total_energy_gwh, 2) OVER (PARTITION BY region_id ORDER BY year)) / 2.0
                )
                ELSE NULL
            END as trend_3y_slope,
            -- Calculate linear trend slope over 5 years
            CASE 
                WHEN year >= MIN(year) OVER (PARTITION BY region_id) + 4
                THEN (
                    (total_energy_gwh - LAG(total_energy_gwh, 4) OVER (PARTITION BY region_id ORDER BY year)) / 4.0
                )
                ELSE NULL
            END as trend_5y_slope
        FROM with_yoy_changes
    )
    SELECT 
        region_id,
        level_code,
        cntr_code,
        region_name,
        year,
        total_energy_gwh,
        renewable_energy_gwh,
        fossil_energy_gwh,
        year_index,
        is_first_year,
        is_last_year,
        lag_1_year_total_energy_gwh,
        lag_2_year_total_energy_gwh,
        lag_3_year_total_energy_gwh,
        lag_1_year_renewable_gwh,
        lag_2_year_renewable_gwh,
        yoy_change_total_energy_pct,
        yoy_change_renewable_pct,
        yoy_change_absolute_gwh,
        rolling_3y_mean_total_energy_gwh,
        rolling_5y_mean_total_energy_gwh,
        rolling_3y_mean_renewable_gwh,
        rolling_5y_mean_renewable_gwh,
        trend_3y_slope,
        trend_5y_slope,
        CASE WHEN trend_3y_slope > 0 THEN TRUE ELSE FALSE END as is_increasing_trend,
        CASE WHEN trend_3y_slope < 0 THEN TRUE ELSE FALSE END as is_decreasing_trend,
        CASE WHEN total_energy_gwh > 0 THEN (renewable_energy_gwh / total_energy_gwh * 100) ELSE 0 END as pct_renewable,
        CASE WHEN total_energy_gwh > 0 THEN (fossil_energy_gwh / total_energy_gwh * 100) ELSE 0 END as pct_fossil,
        feature_count
    FROM with_trends
    ORDER BY region_id, year
    """
    
    result = db_hook.get_records(extract_sql)
    
    extract_stats = {
        'total_records': len(result),
        'unique_regions': len(set(row[0] for row in result)) if result else 0,
        'unique_years': len(set(row[4] for row in result)) if result else 0,
    }
    
    logging.info(f"Extracted {extract_stats['total_records']} forecasting feature records")
    logging.info(f"Unique regions: {extract_stats['unique_regions']}, years: {extract_stats['unique_years']}")
    
    return extract_stats


def load_forecasting_dataset(**context):
    """
    Load forecasting features into ml_dataset_forecasting_v1 table.
    """
    ti = context['ti']
    extract_stats = ti.xcom_pull(task_ids='extract_forecasting_features')
    
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    logging.info("Loading forecasting features into ml_dataset_forecasting_v1...")
    
    # Load features with spatial data
    insert_sql = """
    WITH energy_by_region_year AS (
        SELECT 
            r.region_id,
            r.level_code,
            r.cntr_code,
            r.name_latn as region_name,
            e.year,
            SUM(e.value_gwh) as total_energy_gwh,
            SUM(CASE WHEN p.renewable_flag = TRUE THEN e.value_gwh ELSE 0 END) as renewable_energy_gwh,
            SUM(CASE WHEN p.fossil_flag = TRUE THEN e.value_gwh ELSE 0 END) as fossil_energy_gwh,
            COUNT(*) as feature_count
        FROM fact_energy_annual e
        INNER JOIN dim_region r ON e.country_code = r.cntr_code AND r.level_code = 0
        LEFT JOIN dim_energy_product p ON e.product_code = p.product_code
        WHERE e.value_gwh IS NOT NULL AND e.value_gwh > 0
        GROUP BY r.region_id, r.level_code, r.cntr_code, r.name_latn, e.year
    ),
    with_lags AS (
        SELECT 
            *,
            LAG(total_energy_gwh, 1) OVER (PARTITION BY region_id ORDER BY year) as lag_1_year_total_energy_gwh,
            LAG(total_energy_gwh, 2) OVER (PARTITION BY region_id ORDER BY year) as lag_2_year_total_energy_gwh,
            LAG(total_energy_gwh, 3) OVER (PARTITION BY region_id ORDER BY year) as lag_3_year_total_energy_gwh,
            LAG(renewable_energy_gwh, 1) OVER (PARTITION BY region_id ORDER BY year) as lag_1_year_renewable_gwh,
            LAG(renewable_energy_gwh, 2) OVER (PARTITION BY region_id ORDER BY year) as lag_2_year_renewable_gwh,
            year - MIN(year) OVER (PARTITION BY region_id) as year_index,
            year = MIN(year) OVER (PARTITION BY region_id) as is_first_year,
            year = MAX(year) OVER (PARTITION BY region_id) as is_last_year
        FROM energy_by_region_year
    ),
    with_rolling_stats AS (
        SELECT 
            *,
            AVG(total_energy_gwh) OVER (
                PARTITION BY region_id 
                ORDER BY year 
                ROWS BETWEEN 2 PRECEDING AND CURRENT ROW
            ) as rolling_3y_mean_total_energy_gwh,
            AVG(total_energy_gwh) OVER (
                PARTITION BY region_id 
                ORDER BY year 
                ROWS BETWEEN 4 PRECEDING AND CURRENT ROW
            ) as rolling_5y_mean_total_energy_gwh,
            AVG(renewable_energy_gwh) OVER (
                PARTITION BY region_id 
                ORDER BY year 
                ROWS BETWEEN 2 PRECEDING AND CURRENT ROW
            ) as rolling_3y_mean_renewable_gwh,
            AVG(renewable_energy_gwh) OVER (
                PARTITION BY region_id 
                ORDER BY year 
                ROWS BETWEEN 4 PRECEDING AND CURRENT ROW
            ) as rolling_5y_mean_renewable_gwh
        FROM with_lags
    ),
    with_yoy_changes AS (
        SELECT 
            *,
            CASE 
                WHEN lag_1_year_total_energy_gwh > 0 
                THEN ((total_energy_gwh - lag_1_year_total_energy_gwh) / lag_1_year_total_energy_gwh * 100)
                ELSE NULL 
            END as yoy_change_total_energy_pct,
            CASE 
                WHEN lag_1_year_renewable_gwh > 0 
                THEN ((renewable_energy_gwh - lag_1_year_renewable_gwh) / lag_1_year_renewable_gwh * 100)
                ELSE NULL 
            END as yoy_change_renewable_pct,
            (total_energy_gwh - lag_1_year_total_energy_gwh) as yoy_change_absolute_gwh
        FROM with_rolling_stats
    ),
    with_trends AS (
        SELECT 
            *,
            CASE 
                WHEN year >= MIN(year) OVER (PARTITION BY region_id) + 2
                THEN (
                    (total_energy_gwh - LAG(total_energy_gwh, 2) OVER (PARTITION BY region_id ORDER BY year)) / 2.0
                )
                ELSE NULL
            END as trend_3y_slope,
            CASE 
                WHEN year >= MIN(year) OVER (PARTITION BY region_id) + 4
                THEN (
                    (total_energy_gwh - LAG(total_energy_gwh, 4) OVER (PARTITION BY region_id ORDER BY year)) / 4.0
                )
                ELSE NULL
            END as trend_5y_slope
        FROM with_yoy_changes
    ),
    region_spatial AS (
        SELECT 
            r.region_id,
            r.area_km2
        FROM dim_region r
        WHERE r.level_code = 0
    )
    INSERT INTO ml_dataset_forecasting_v1 (
        region_id, level_code, cntr_code, region_name, year,
        total_energy_gwh, renewable_energy_gwh, fossil_energy_gwh,
        year_index, is_first_year, is_last_year,
        lag_1_year_total_energy_gwh, lag_2_year_total_energy_gwh, lag_3_year_total_energy_gwh,
        lag_1_year_renewable_gwh, lag_2_year_renewable_gwh,
        yoy_change_total_energy_pct, yoy_change_renewable_pct, yoy_change_absolute_gwh,
        rolling_3y_mean_total_energy_gwh, rolling_5y_mean_total_energy_gwh,
        rolling_3y_mean_renewable_gwh, rolling_5y_mean_renewable_gwh,
        trend_3y_slope, trend_5y_slope,
        is_increasing_trend, is_decreasing_trend,
        pct_renewable, pct_fossil,
        area_km2, energy_density_gwh_per_km2,
        feature_count, updated_at
    )
    SELECT 
        e.region_id,
        e.level_code,
        e.cntr_code,
        e.region_name,
        e.year,
        e.total_energy_gwh,
        e.renewable_energy_gwh,
        e.fossil_energy_gwh,
        e.year_index,
        e.is_first_year,
        e.is_last_year,
        e.lag_1_year_total_energy_gwh,
        e.lag_2_year_total_energy_gwh,
        e.lag_3_year_total_energy_gwh,
        e.lag_1_year_renewable_gwh,
        e.lag_2_year_renewable_gwh,
        e.yoy_change_total_energy_pct,
        e.yoy_change_renewable_pct,
        e.yoy_change_absolute_gwh,
        e.rolling_3y_mean_total_energy_gwh,
        e.rolling_5y_mean_total_energy_gwh,
        e.rolling_3y_mean_renewable_gwh,
        e.rolling_5y_mean_renewable_gwh,
        e.trend_3y_slope,
        e.trend_5y_slope,
        CASE WHEN e.trend_3y_slope > 0 THEN TRUE ELSE FALSE END,
        CASE WHEN e.trend_3y_slope < 0 THEN TRUE ELSE FALSE END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.renewable_energy_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.fossil_energy_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        s.area_km2,
        CASE WHEN s.area_km2 > 0 THEN (e.total_energy_gwh / s.area_km2) ELSE NULL END,
        e.feature_count,
        NOW()
    FROM with_trends e
    INNER JOIN region_spatial s ON e.region_id = s.region_id
    ON CONFLICT (region_id, year) 
    DO UPDATE SET
        total_energy_gwh = EXCLUDED.total_energy_gwh,
        renewable_energy_gwh = EXCLUDED.renewable_energy_gwh,
        fossil_energy_gwh = EXCLUDED.fossil_energy_gwh,
        year_index = EXCLUDED.year_index,
        is_first_year = EXCLUDED.is_first_year,
        is_last_year = EXCLUDED.is_last_year,
        lag_1_year_total_energy_gwh = EXCLUDED.lag_1_year_total_energy_gwh,
        lag_2_year_total_energy_gwh = EXCLUDED.lag_2_year_total_energy_gwh,
        lag_3_year_total_energy_gwh = EXCLUDED.lag_3_year_total_energy_gwh,
        lag_1_year_renewable_gwh = EXCLUDED.lag_1_year_renewable_gwh,
        lag_2_year_renewable_gwh = EXCLUDED.lag_2_year_renewable_gwh,
        yoy_change_total_energy_pct = EXCLUDED.yoy_change_total_energy_pct,
        yoy_change_renewable_pct = EXCLUDED.yoy_change_renewable_pct,
        yoy_change_absolute_gwh = EXCLUDED.yoy_change_absolute_gwh,
        rolling_3y_mean_total_energy_gwh = EXCLUDED.rolling_3y_mean_total_energy_gwh,
        rolling_5y_mean_total_energy_gwh = EXCLUDED.rolling_5y_mean_total_energy_gwh,
        rolling_3y_mean_renewable_gwh = EXCLUDED.rolling_3y_mean_renewable_gwh,
        rolling_5y_mean_renewable_gwh = EXCLUDED.rolling_5y_mean_renewable_gwh,
        trend_3y_slope = EXCLUDED.trend_3y_slope,
        trend_5y_slope = EXCLUDED.trend_5y_slope,
        is_increasing_trend = EXCLUDED.is_increasing_trend,
        is_decreasing_trend = EXCLUDED.is_decreasing_trend,
        pct_renewable = EXCLUDED.pct_renewable,
        pct_fossil = EXCLUDED.pct_fossil,
        area_km2 = EXCLUDED.area_km2,
        energy_density_gwh_per_km2 = EXCLUDED.energy_density_gwh_per_km2,
        feature_count = EXCLUDED.feature_count,
        updated_at = NOW()
    """
    
    db_hook.run(insert_sql)
    
    # Get statistics
    stats_sql = """
        SELECT 
            COUNT(*) as total_records,
            COUNT(DISTINCT region_id) as unique_regions,
            COUNT(DISTINCT year) as unique_years,
            MIN(year) as min_year,
            MAX(year) as max_year,
            AVG(total_energy_gwh) as avg_energy_gwh,
            AVG(yoy_change_total_energy_pct) as avg_yoy_change_pct
        FROM ml_dataset_forecasting_v1
    """
    stats = db_hook.get_first(stats_sql)
    
    load_stats = {
        'total_records': stats[0],
        'unique_regions': stats[1],
        'unique_years': stats[2],
        'min_year': stats[3],
        'max_year': stats[4],
        'avg_energy_gwh': float(stats[5]) if stats[5] else 0,
        'avg_yoy_change_pct': float(stats[6]) if stats[6] else 0,
    }
    
    logging.info(f"Dataset loaded: {load_stats['total_records']} records, "
                 f"{load_stats['unique_regions']} regions, "
                 f"{load_stats['unique_years']} years")
    
    return load_stats


def validate_forecasting_dataset(**context):
    """
    Validate the forecasting dataset.
    """
    ti = context['ti']
    load_stats = ti.xcom_pull(task_ids='load_forecasting_dataset')
    
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    logging.info("Validating forecasting dataset...")
    
    validation_queries = {
        'null_checks': """
            SELECT 
                COUNT(*) as total,
                SUM(CASE WHEN total_energy_gwh IS NULL THEN 1 ELSE 0 END) as null_energy,
                SUM(CASE WHEN lag_1_year_total_energy_gwh IS NULL THEN 1 ELSE 0 END) as null_lag1,
                SUM(CASE WHEN rolling_3y_mean_total_energy_gwh IS NULL THEN 1 ELSE 0 END) as null_rolling3y
            FROM ml_dataset_forecasting_v1
        """,
        'trend_validation': """
            SELECT 
                COUNT(*) as total,
                SUM(CASE WHEN is_increasing_trend = is_decreasing_trend THEN 1 ELSE 0 END) as invalid_trend_flags
            FROM ml_dataset_forecasting_v1
        """,
        'feature_completeness': """
            SELECT 
                COUNT(*) as total,
                AVG(CASE WHEN total_energy_gwh > 0 THEN 1.0 ELSE 0.0 END) * 100 as pct_with_energy,
                AVG(CASE WHEN lag_1_year_total_energy_gwh IS NOT NULL THEN 1.0 ELSE 0.0 END) * 100 as pct_with_lag1,
                AVG(CASE WHEN rolling_3y_mean_total_energy_gwh IS NOT NULL THEN 1.0 ELSE 0.0 END) * 100 as pct_with_rolling3y
            FROM ml_dataset_forecasting_v1
        """,
    }
    
    validation_results = {}
    
    # Null checks
    null_result = db_hook.get_first(validation_queries['null_checks'])
    validation_results['null_checks'] = {
        'total': null_result[0],
        'null_energy': null_result[1],
        'null_lag1': null_result[2],
        'null_rolling3y': null_result[3]
    }
    
    # Trend validation
    trend_result = db_hook.get_first(validation_queries['trend_validation'])
    validation_results['trend_validation'] = {
        'total': trend_result[0],
        'invalid_trend_flags': trend_result[1]
    }
    
    # Feature completeness
    completeness_result = db_hook.get_first(validation_queries['feature_completeness'])
    validation_results['feature_completeness'] = {
        'total': completeness_result[0],
        'pct_with_energy': float(completeness_result[1]) if completeness_result[1] else 0,
        'pct_with_lag1': float(completeness_result[2]) if completeness_result[2] else 0,
        'pct_with_rolling3y': float(completeness_result[3]) if completeness_result[3] else 0
    }
    
    # Check for validation issues
    issues = []
    if validation_results['null_checks']['null_energy'] > 0:
        issues.append(f"Found {validation_results['null_checks']['null_energy']} records with null energy")
    if validation_results['trend_validation']['invalid_trend_flags'] > 0:
        issues.append(f"Found {validation_results['trend_validation']['invalid_trend_flags']} records with invalid trend flags")
    
    validation_results['issues'] = issues
    validation_results['is_valid'] = len(issues) == 0
    
    if issues:
        logging.warning(f"Validation issues found: {', '.join(issues)}")
    else:
        logging.info("Dataset validation passed")
    
    return validation_results


def log_dataset_creation(**context):
    """
    Log dataset creation to metadata table.
    """
    ti = context['ti']
    load_stats = ti.xcom_pull(task_ids='load_forecasting_dataset')
    validation_results = ti.xcom_pull(task_ids='validate_forecasting_dataset')
    
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    logging.info("Logging dataset creation to metadata table...")
    
    log_sql = """
    INSERT INTO meta_ingestion_log (
        source_system,
        table_name,
        ingestion_timestamp,
        records_inserted,
        records_updated,
        status,
        metadata
    ) VALUES (
        'ml_pipeline',
        'ml_dataset_forecasting_v1',
        NOW(),
        %s,
        0,
        %s,
        %s::jsonb
    )
    """
    
    metadata = {
        'unique_regions': load_stats['unique_regions'],
        'unique_years': load_stats['unique_years'],
        'min_year': load_stats['min_year'],
        'max_year': load_stats['max_year'],
        'avg_energy_gwh': load_stats['avg_energy_gwh'],
        'avg_yoy_change_pct': load_stats['avg_yoy_change_pct'],
        'validation_passed': validation_results['is_valid'],
        'validation_issues': validation_results['issues']
    }
    
    import json
    db_hook.run(log_sql, parameters=(
        load_stats['total_records'],
        'success' if validation_results['is_valid'] else 'warning',
        json.dumps(metadata)
    ))
    
    logging.info("✅ Dataset creation logged to metadata table")
    
    return {'logged': True}


# Task definitions
create_table_task = PythonOperator(
    task_id='create_forecasting_dataset_table',
    python_callable=create_forecasting_dataset_table,
    dag=dag,
)

extract_features_task = PythonOperator(
    task_id='extract_forecasting_features',
    python_callable=extract_forecasting_features,
    dag=dag,
)

load_dataset_task = PythonOperator(
    task_id='load_forecasting_dataset',
    python_callable=load_forecasting_dataset,
    dag=dag,
)

validate_dataset_task = PythonOperator(
    task_id='validate_forecasting_dataset',
    python_callable=validate_forecasting_dataset,
    dag=dag,
)

log_creation_task = PythonOperator(
    task_id='log_dataset_creation',
    python_callable=log_dataset_creation,
    dag=dag,
)

# Task dependencies
create_table_task >> extract_features_task >> load_dataset_task >> validate_dataset_task >> log_creation_task
