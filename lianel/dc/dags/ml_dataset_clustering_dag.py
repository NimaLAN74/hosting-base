"""
ML Clustering Dataset Creation DAG

This DAG creates a clustering dataset by extracting features from fact_energy_annual
and joining with dim_region for spatial features. The dataset is designed for
regional energy mix clustering analysis.

Schedule: Weekly (every Sunday at 04:00 UTC, after harmonization)
Manual trigger: Yes
"""
from datetime import datetime, timedelta
from airflow import DAG
from airflow.operators.python import PythonOperator
from airflow.providers.postgres.hooks.postgres import PostgresHook
import logging

default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'start_date': datetime(2026, 1, 1),
    'email_on_failure': False,
    'email_on_retry': False,
    'retries': 2,
    'retry_delay': timedelta(minutes=10),
    'execution_timeout': timedelta(hours=3),
}

dag = DAG(
    'ml_dataset_clustering',
    default_args=default_args,
    description='Create ML clustering dataset from energy and spatial data',
    schedule='0 4 * * 0',  # Every Sunday at 04:00 UTC (after harmonization)
    catchup=False,
    tags=['ml', 'dataset', 'clustering', 'feature-engineering'],
    max_active_runs=1,
)


def create_clustering_dataset_table(**context) -> None:
    """
    Create ml_dataset_clustering_v1 table if it doesn't exist.
    """
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    create_table_sql = """
    CREATE TABLE IF NOT EXISTS ml_dataset_clustering_v1 (
        -- Primary key
        dataset_id BIGSERIAL PRIMARY KEY,
        
        -- Region identifier (NUTS2 level)
        region_id VARCHAR(10) NOT NULL,
        level_code INTEGER NOT NULL,
        cntr_code VARCHAR(2) NOT NULL,
        region_name VARCHAR(255),
        
        -- Time dimension
        year INTEGER NOT NULL,
        
        -- Energy mix features (aggregated by region and year)
        -- Total energy consumption
        total_energy_gwh DOUBLE PRECISION,
        
        -- Energy by product type (percentages)
        pct_renewable DOUBLE PRECISION,
        pct_fossil DOUBLE PRECISION,
        pct_nuclear DOUBLE PRECISION,
        pct_hydro DOUBLE PRECISION,
        pct_wind DOUBLE PRECISION,
        pct_solar DOUBLE PRECISION,
        pct_biomass DOUBLE PRECISION,
        
        -- Energy by flow type (percentages)
        pct_production DOUBLE PRECISION,
        pct_imports DOUBLE PRECISION,
        pct_exports DOUBLE PRECISION,
        pct_final_consumption DOUBLE PRECISION,
        pct_transformation DOUBLE PRECISION,
        
        -- Spatial features (from dim_region)
        area_km2 DOUBLE PRECISION,
        mount_type VARCHAR(50),
        urbn_type VARCHAR(50),
        coast_type VARCHAR(50),
        
        -- Derived features
        energy_density_gwh_per_km2 DOUBLE PRECISION,  -- Total energy / area
        
        -- Metadata
        feature_count INTEGER,  -- Number of energy records aggregated
        created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
        updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
        
        -- Constraints
        CONSTRAINT uk_clustering_region_year UNIQUE (region_id, year),
        CONSTRAINT fk_clustering_region FOREIGN KEY (region_id) 
            REFERENCES dim_region(region_id) ON DELETE RESTRICT
    );
    
    -- Indexes for fast queries
    CREATE INDEX IF NOT EXISTS idx_clustering_region ON ml_dataset_clustering_v1(region_id);
    CREATE INDEX IF NOT EXISTS idx_clustering_year ON ml_dataset_clustering_v1(year);
    CREATE INDEX IF NOT EXISTS idx_clustering_country ON ml_dataset_clustering_v1(cntr_code);
    CREATE INDEX IF NOT EXISTS idx_clustering_year_region ON ml_dataset_clustering_v1(year, region_id);
    """
    
    logging.info("Creating ml_dataset_clustering_v1 table if it doesn't exist...")
    db_hook.run(create_table_sql)
    logging.info("Table created/verified successfully")


def extract_energy_features(**context) -> dict:
    """
    Extract energy features from fact_energy_annual and aggregate by region and year.
    Joins with dim_region to get spatial features.
    """
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    logging.info("Extracting energy features from fact_energy_annual...")
    
    # First, we need to map country codes to NUTS2 regions
    # For now, we'll use NUTS0 (country level) as the region
    # In the future, we can enhance this to use actual NUTS2 regions
    
    extract_sql = """
    WITH energy_aggregated AS (
        -- Aggregate energy data by country and year
        SELECT 
            e.country_code,
            e.year,
            -- Total energy
            SUM(e.value_gwh) as total_energy_gwh,
            
            -- Energy by product type (using product codes)
            SUM(CASE WHEN p.renewable_flag = TRUE THEN e.value_gwh ELSE 0 END) as renewable_gwh,
            SUM(CASE WHEN p.fossil_flag = TRUE THEN e.value_gwh ELSE 0 END) as fossil_gwh,
            SUM(CASE WHEN p.product_code LIKE '%NUCLEAR%' OR p.product_code = 'N900H' THEN e.value_gwh ELSE 0 END) as nuclear_gwh,
            SUM(CASE WHEN p.product_code LIKE '%HYDRO%' OR p.product_code LIKE '%WATER%' THEN e.value_gwh ELSE 0 END) as hydro_gwh,
            SUM(CASE WHEN p.product_code LIKE '%WIND%' THEN e.value_gwh ELSE 0 END) as wind_gwh,
            SUM(CASE WHEN p.product_code LIKE '%SOLAR%' OR p.product_code LIKE '%SUN%' THEN e.value_gwh ELSE 0 END) as solar_gwh,
            SUM(CASE WHEN p.product_code LIKE '%BIOMASS%' OR p.product_code LIKE '%BIO%' THEN e.value_gwh ELSE 0 END) as biomass_gwh,
            
            -- Energy by flow type
            SUM(CASE WHEN f.flow_code LIKE '%PRODUCTION%' OR f.flow_code = 'P1000' THEN e.value_gwh ELSE 0 END) as production_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%IMPORT%' OR f.flow_code = 'IMP' THEN e.value_gwh ELSE 0 END) as imports_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%EXPORT%' OR f.flow_code = 'EXP' THEN e.value_gwh ELSE 0 END) as exports_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%CONSUMPTION%' OR f.flow_code LIKE '%FC%' THEN e.value_gwh ELSE 0 END) as final_consumption_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%TRANSFORMATION%' OR f.flow_code LIKE '%TRANS%' THEN e.value_gwh ELSE 0 END) as transformation_gwh,
            
            -- Count of records
            COUNT(*) as feature_count
            
        FROM fact_energy_annual e
        LEFT JOIN dim_energy_product p ON e.product_code = p.product_code
        LEFT JOIN dim_energy_flow f ON e.flow_code = f.flow_code
        WHERE e.value_gwh IS NOT NULL AND e.value_gwh > 0
        GROUP BY e.country_code, e.year
    ),
    region_features AS (
        -- Get NUTS0 regions (country level) with spatial features
        SELECT 
            r.region_id,
            r.level_code,
            r.cntr_code,
            r.name_latn as region_name,
            r.area_km2,
            r.mount_type,
            r.urbn_type,
            r.coast_type
        FROM dim_region r
        WHERE r.level_code = 0  -- NUTS0 = country level
    )
    SELECT 
        r.region_id,
        r.level_code,
        r.cntr_code,
        r.region_name,
        e.year,
        e.total_energy_gwh,
        
        -- Calculate percentages
        CASE WHEN e.total_energy_gwh > 0 
            THEN (e.renewable_gwh / e.total_energy_gwh * 100) 
            ELSE 0 END as pct_renewable,
        CASE WHEN e.total_energy_gwh > 0 
            THEN (e.fossil_gwh / e.total_energy_gwh * 100) 
            ELSE 0 END as pct_fossil,
        CASE WHEN e.total_energy_gwh > 0 
            THEN (e.nuclear_gwh / e.total_energy_gwh * 100) 
            ELSE 0 END as pct_nuclear,
        CASE WHEN e.total_energy_gwh > 0 
            THEN (e.hydro_gwh / e.total_energy_gwh * 100) 
            ELSE 0 END as pct_hydro,
        CASE WHEN e.total_energy_gwh > 0 
            THEN (e.wind_gwh / e.total_energy_gwh * 100) 
            ELSE 0 END as pct_wind,
        CASE WHEN e.total_energy_gwh > 0 
            THEN (e.solar_gwh / e.total_energy_gwh * 100) 
            ELSE 0 END as pct_solar,
        CASE WHEN e.total_energy_gwh > 0 
            THEN (e.biomass_gwh / e.total_energy_gwh * 100) 
            ELSE 0 END as pct_biomass,
        
        CASE WHEN e.total_energy_gwh > 0 
            THEN (e.production_gwh / e.total_energy_gwh * 100) 
            ELSE 0 END as pct_production,
        CASE WHEN e.total_energy_gwh > 0 
            THEN (e.imports_gwh / e.total_energy_gwh * 100) 
            ELSE 0 END as pct_imports,
        CASE WHEN e.total_energy_gwh > 0 
            THEN (e.exports_gwh / e.total_energy_gwh * 100) 
            ELSE 0 END as pct_exports,
        CASE WHEN e.total_energy_gwh > 0 
            THEN (e.final_consumption_gwh / e.total_energy_gwh * 100) 
            ELSE 0 END as pct_final_consumption,
        CASE WHEN e.total_energy_gwh > 0 
            THEN (e.transformation_gwh / e.total_energy_gwh * 100) 
            ELSE 0 END as pct_transformation,
        
        r.area_km2,
        r.mount_type,
        r.urbn_type,
        r.coast_type,
        
        -- Energy density (GWh per km²)
        CASE WHEN r.area_km2 > 0 
            THEN (e.total_energy_gwh / r.area_km2) 
            ELSE NULL END as energy_density_gwh_per_km2,
        
        e.feature_count
        
    FROM energy_aggregated e
    INNER JOIN region_features r ON e.country_code = r.cntr_code
    ORDER BY r.cntr_code, e.year
    """
    
    results = db_hook.get_records(extract_sql)
    
    feature_count = len(results)
    logging.info(f"Extracted {feature_count} feature records")
    
    return {
        'feature_count': feature_count,
        'extract_sql': extract_sql,
    }


def load_clustering_dataset(**context) -> dict:
    """
    Load extracted features into ml_dataset_clustering_v1 table.
    """
    ti = context['ti']
    extract_info = ti.xcom_pull(task_ids='extract_energy_features')
    
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    logging.info("Loading features into ml_dataset_clustering_v1...")
    
    # Use the same SQL but insert into the table
    insert_sql = """
    WITH energy_aggregated AS (
        SELECT 
            e.country_code,
            e.year,
            SUM(e.value_gwh) as total_energy_gwh,
            SUM(CASE WHEN p.renewable_flag = TRUE THEN e.value_gwh ELSE 0 END) as renewable_gwh,
            SUM(CASE WHEN p.fossil_flag = TRUE THEN e.value_gwh ELSE 0 END) as fossil_gwh,
            SUM(CASE WHEN p.product_code LIKE '%NUCLEAR%' OR p.product_code = 'N900H' THEN e.value_gwh ELSE 0 END) as nuclear_gwh,
            SUM(CASE WHEN p.product_code LIKE '%HYDRO%' OR p.product_code LIKE '%WATER%' THEN e.value_gwh ELSE 0 END) as hydro_gwh,
            SUM(CASE WHEN p.product_code LIKE '%WIND%' THEN e.value_gwh ELSE 0 END) as wind_gwh,
            SUM(CASE WHEN p.product_code LIKE '%SOLAR%' OR p.product_code LIKE '%SUN%' THEN e.value_gwh ELSE 0 END) as solar_gwh,
            SUM(CASE WHEN p.product_code LIKE '%BIOMASS%' OR p.product_code LIKE '%BIO%' THEN e.value_gwh ELSE 0 END) as biomass_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%PRODUCTION%' OR f.flow_code = 'P1000' THEN e.value_gwh ELSE 0 END) as production_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%IMPORT%' OR f.flow_code = 'IMP' THEN e.value_gwh ELSE 0 END) as imports_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%EXPORT%' OR f.flow_code = 'EXP' THEN e.value_gwh ELSE 0 END) as exports_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%CONSUMPTION%' OR f.flow_code LIKE '%FC%' THEN e.value_gwh ELSE 0 END) as final_consumption_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%TRANSFORMATION%' OR f.flow_code LIKE '%TRANS%' THEN e.value_gwh ELSE 0 END) as transformation_gwh,
            COUNT(*) as feature_count
        FROM fact_energy_annual e
        LEFT JOIN dim_energy_product p ON e.product_code = p.product_code
        LEFT JOIN dim_energy_flow f ON e.flow_code = f.flow_code
        WHERE e.value_gwh IS NOT NULL AND e.value_gwh > 0
        GROUP BY e.country_code, e.year
    ),
    region_features AS (
        SELECT 
            r.region_id,
            r.level_code,
            r.cntr_code,
            r.name_latn as region_name,
            r.area_km2,
            r.mount_type,
            r.urbn_type,
            r.coast_type
        FROM dim_region r
        WHERE r.level_code = 0
    )
    INSERT INTO ml_dataset_clustering_v1 (
        region_id, level_code, cntr_code, region_name, year,
        total_energy_gwh,
        pct_renewable, pct_fossil, pct_nuclear, pct_hydro, pct_wind, pct_solar, pct_biomass,
        pct_production, pct_imports, pct_exports, pct_final_consumption, pct_transformation,
        area_km2, mount_type, urbn_type, coast_type,
        energy_density_gwh_per_km2, feature_count, updated_at
    )
    SELECT 
        r.region_id,
        r.level_code,
        r.cntr_code,
        r.region_name,
        e.year,
        e.total_energy_gwh,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.renewable_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.fossil_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.nuclear_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.hydro_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.wind_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.solar_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.biomass_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.production_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.imports_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.exports_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.final_consumption_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.transformation_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        r.area_km2,
        r.mount_type,
        r.urbn_type,
        r.coast_type,
        CASE WHEN r.area_km2 > 0 THEN (e.total_energy_gwh / r.area_km2) ELSE NULL END,
        e.feature_count,
        NOW()
    FROM energy_aggregated e
    INNER JOIN region_features r ON e.country_code = r.cntr_code
    ON CONFLICT (region_id, year) 
    DO UPDATE SET
        total_energy_gwh = EXCLUDED.total_energy_gwh,
        pct_renewable = EXCLUDED.pct_renewable,
        pct_fossil = EXCLUDED.pct_fossil,
        pct_nuclear = EXCLUDED.pct_nuclear,
        pct_hydro = EXCLUDED.pct_hydro,
        pct_wind = EXCLUDED.pct_wind,
        pct_solar = EXCLUDED.pct_solar,
        pct_biomass = EXCLUDED.pct_biomass,
        pct_production = EXCLUDED.pct_production,
        pct_imports = EXCLUDED.pct_imports,
        pct_exports = EXCLUDED.pct_exports,
        pct_final_consumption = EXCLUDED.pct_final_consumption,
        pct_transformation = EXCLUDED.pct_transformation,
        area_km2 = EXCLUDED.area_km2,
        mount_type = EXCLUDED.mount_type,
        urbn_type = EXCLUDED.urbn_type,
        coast_type = EXCLUDED.coast_type,
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
            SUM(total_energy_gwh) as total_energy_sum_gwh
        FROM ml_dataset_clustering_v1
    """
    stats = db_hook.get_first(stats_sql)
    
    load_stats = {
        'total_records': stats[0],
        'unique_regions': stats[1],
        'unique_years': stats[2],
        'min_year': stats[3],
        'max_year': stats[4],
        'avg_energy_gwh': float(stats[5]) if stats[5] else 0,
        'total_energy_sum_gwh': float(stats[6]) if stats[6] else 0,
    }
    
    logging.info(f"Dataset loaded: {load_stats['total_records']} records, "
                 f"{load_stats['unique_regions']} regions, "
                 f"{load_stats['unique_years']} years")
    
    return load_stats


def validate_clustering_dataset(**context) -> dict:
    """
    Validate the clustering dataset: check completeness, feature ranges, etc.
    """
    ti = context['ti']
    load_stats = ti.xcom_pull(task_ids='load_clustering_dataset')
    
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    logging.info("Validating clustering dataset...")
    
    validation_queries = {
        'null_checks': """
            SELECT 
                COUNT(*) as total,
                SUM(CASE WHEN total_energy_gwh IS NULL THEN 1 ELSE 0 END) as null_energy,
                SUM(CASE WHEN area_km2 IS NULL THEN 1 ELSE 0 END) as null_area,
                SUM(CASE WHEN pct_renewable IS NULL THEN 1 ELSE 0 END) as null_pct_renewable
            FROM ml_dataset_clustering_v1
        """,
        'percentage_ranges': """
            SELECT 
                COUNT(*) as total,
                SUM(CASE WHEN pct_renewable < 0 OR pct_renewable > 100 THEN 1 ELSE 0 END) as invalid_pct_renewable,
                SUM(CASE WHEN pct_fossil < 0 OR pct_fossil > 100 THEN 1 ELSE 0 END) as invalid_pct_fossil
            FROM ml_dataset_clustering_v1
        """,
        'feature_completeness': """
            SELECT 
                COUNT(*) as total,
                AVG(CASE WHEN total_energy_gwh > 0 THEN 1.0 ELSE 0.0 END) * 100 as pct_with_energy,
                AVG(CASE WHEN area_km2 > 0 THEN 1.0 ELSE 0.0 END) * 100 as pct_with_area
            FROM ml_dataset_clustering_v1
        """,
    }
    
    validation_results = {}
    for check_name, query in validation_queries.items():
        result = db_hook.get_first(query)
        validation_results[check_name] = {
            'total': result[0],
            **{f'check_{i}': result[i] for i in range(1, len(result))}
        }
    
    # Check for any validation issues
    issues = []
    if validation_results['null_checks']['null_energy'] > 0:
        issues.append(f"Found {validation_results['null_checks']['null_energy']} records with null energy")
    if validation_results['percentage_ranges']['invalid_pct_renewable'] > 0:
        issues.append(f"Found {validation_results['percentage_ranges']['invalid_pct_renewable']} records with invalid renewable percentage")
    
    validation_results['issues'] = issues
    validation_results['is_valid'] = len(issues) == 0
    
    if issues:
        logging.warning(f"Validation issues found: {', '.join(issues)}")
    else:
        logging.info("Dataset validation passed")
    
    return validation_results


def log_dataset_creation(**context) -> None:
    """Log dataset creation to metadata table"""
    ti = context['ti']
    load_stats = ti.xcom_pull(task_ids='load_clustering_dataset')
    validation_results = ti.xcom_pull(task_ids='validate_clustering_dataset')
    
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    import json
    summary = {
        'dataset_type': 'clustering',
        'dataset_version': 'v1',
        'load_stats': load_stats,
        'validation': validation_results,
    }
    
    insert_sql = """
        INSERT INTO meta_ingestion_log (
            source_system, table_name, status, records_inserted, records_updated,
            metadata
        ) VALUES (
            %s, %s, %s, %s, %s, %s::jsonb
        )
    """
    
    status = 'success' if validation_results.get('is_valid', False) else 'partial'
    
    db_hook.run(insert_sql, parameters=(
        'ml_pipeline',
        'ml_dataset_clustering_v1',
        status,
        load_stats.get('total_records', 0),
        0,  # No updates, only inserts
        json.dumps(summary),
    ))
    
    logging.info(f"ML dataset creation logged: {load_stats.get('total_records', 0)} records")


# Create tasks
create_table_task = PythonOperator(
    task_id='create_clustering_dataset_table',
    python_callable=create_clustering_dataset_table,
    dag=dag,
)

extract_task = PythonOperator(
    task_id='extract_energy_features',
    python_callable=extract_energy_features,
    dag=dag,
)

load_task = PythonOperator(
    task_id='load_clustering_dataset',
    python_callable=load_clustering_dataset,
    dag=dag,
)

validate_task = PythonOperator(
    task_id='validate_clustering_dataset',
    python_callable=validate_clustering_dataset,
    dag=dag,
)

log_task = PythonOperator(
    task_id='log_dataset_creation',
    python_callable=log_dataset_creation,
    dag=dag,
)

# Set task dependencies
create_table_task >> extract_task >> load_task >> validate_task >> log_task
