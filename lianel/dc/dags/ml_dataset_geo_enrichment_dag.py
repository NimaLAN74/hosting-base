"""
ML Geo-Enrichment Dataset DAG

Creates a geo-enriched dataset combining energy data with spatial features.
This dataset is optimized for spatial analysis and regional segmentation.

Table: ml_dataset_geo_enrichment_v1
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
    'ml_dataset_geo_enrichment',
    default_args=default_args,
    description='Create ML geo-enrichment dataset combining energy and spatial data',
    schedule='0 6 * * 0',  # Every Sunday at 06:00 UTC (after forecasting dataset)
    start_date=datetime(2026, 1, 1),
    catchup=False,
    tags=['ml', 'dataset', 'geo-enrichment', 'spatial', 'lianel'],
    max_active_runs=1,
)


def create_geo_enrichment_dataset_table(**context):
    """
    Create ml_dataset_geo_enrichment_v1 table if it doesn't exist.
    Also add OSM columns if they're missing (for existing tables).
    """
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    logging.info("Creating ml_dataset_geo_enrichment_v1 table if it doesn't exist...")
    
    create_table_sql = """
    CREATE TABLE IF NOT EXISTS ml_dataset_geo_enrichment_v1 (
        dataset_id BIGSERIAL PRIMARY KEY,
        region_id VARCHAR(10) NOT NULL,
        level_code INTEGER NOT NULL,
        cntr_code VARCHAR(2) NOT NULL,
        region_name VARCHAR(255),
        year INTEGER NOT NULL,
        
        -- Energy indicators
        total_energy_gwh NUMERIC(15,3),
        renewable_energy_gwh NUMERIC(15,3),
        fossil_energy_gwh NUMERIC(15,3),
        nuclear_energy_gwh NUMERIC(15,3),
        
        -- Energy mix percentages
        pct_renewable NUMERIC(5,2),
        pct_fossil NUMERIC(5,2),
        pct_nuclear NUMERIC(5,2),
        
        -- Energy flow percentages
        pct_production NUMERIC(5,2),
        pct_imports NUMERIC(5,2),
        pct_exports NUMERIC(5,2),
        pct_final_consumption NUMERIC(5,2),
        
        -- Spatial features
        area_km2 DOUBLE PRECISION,
        mount_type VARCHAR(50),
        urbn_type VARCHAR(50),
        coast_type VARCHAR(50),
        
        -- Derived spatial-energy features
        energy_density_gwh_per_km2 NUMERIC(15,3),
        renewable_density_gwh_per_km2 NUMERIC(15,3),
        fossil_density_gwh_per_km2 NUMERIC(15,3),
        
        -- Energy ratios
        renewable_to_fossil_ratio NUMERIC(10,3),
        production_to_consumption_ratio NUMERIC(10,3),
        imports_to_production_ratio NUMERIC(10,3),
        
        -- Regional characteristics
        is_coastal BOOLEAN,
        is_mountainous BOOLEAN,
        is_urban BOOLEAN,
        is_rural BOOLEAN,
        
        -- OSM Features (from fact_geo_region_features)
        power_plant_count INTEGER,
        power_generator_count INTEGER,
        power_substation_count INTEGER,
        industrial_area_km2 NUMERIC(12,2),
        railway_station_count INTEGER,
        airport_count INTEGER,
        
        -- OSM Feature Densities
        power_plant_density_per_km2 NUMERIC(10,3),
        power_generator_density_per_km2 NUMERIC(10,3),
        power_substation_density_per_km2 NUMERIC(10,3),
        industrial_area_pct NUMERIC(5,2),
        
        -- Metadata
        feature_count INTEGER,
        osm_feature_count INTEGER,
        created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
        updated_at TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW(),
        
        CONSTRAINT uk_geo_enrichment_region_year UNIQUE (region_id, year),
        CONSTRAINT fk_geo_enrichment_region FOREIGN KEY (region_id) 
            REFERENCES dim_region(region_id) ON DELETE RESTRICT
    );
    
    -- Create indexes
    CREATE INDEX IF NOT EXISTS idx_geo_enrichment_region_year 
        ON ml_dataset_geo_enrichment_v1(region_id, year);
    CREATE INDEX IF NOT EXISTS idx_geo_enrichment_year 
        ON ml_dataset_geo_enrichment_v1(year);
    CREATE INDEX IF NOT EXISTS idx_geo_enrichment_cntr_code 
        ON ml_dataset_geo_enrichment_v1(cntr_code);
    CREATE INDEX IF NOT EXISTS idx_geo_enrichment_level 
        ON ml_dataset_geo_enrichment_v1(level_code);
    """
    
    db_hook.run(create_table_sql)
    logging.info("✅ Table ml_dataset_geo_enrichment_v1 created/verified")
    
    return {'table_created': True}


def extract_geo_enrichment_features(**context):
    """
    Extract geo-enrichment features combining energy and spatial data.
    """
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    logging.info("Extracting geo-enrichment features...")
    
    # Extract features with spatial enrichment
    extract_sql = """
    WITH energy_aggregated AS (
        SELECT 
            e.country_code,
            e.year,
            SUM(e.value_gwh) as total_energy_gwh,
            SUM(CASE WHEN p.renewable_flag = TRUE THEN e.value_gwh ELSE 0 END) as renewable_energy_gwh,
            SUM(CASE WHEN p.fossil_flag = TRUE THEN e.value_gwh ELSE 0 END) as fossil_energy_gwh,
            SUM(CASE WHEN p.product_code LIKE '%NUCLEAR%' OR p.product_code = 'N900H' THEN e.value_gwh ELSE 0 END) as nuclear_energy_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%PRODUCTION%' OR f.flow_code = 'P1000' THEN e.value_gwh ELSE 0 END) as production_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%IMPORT%' OR f.flow_code = 'IMP' THEN e.value_gwh ELSE 0 END) as imports_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%EXPORT%' OR f.flow_code = 'EXP' THEN e.value_gwh ELSE 0 END) as exports_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%CONSUMPTION%' OR f.flow_code LIKE '%FC%' THEN e.value_gwh ELSE 0 END) as final_consumption_gwh,
            COUNT(*) as feature_count
        FROM fact_energy_annual e
        LEFT JOIN dim_energy_product p ON e.product_code = p.product_code
        LEFT JOIN dim_energy_flow f ON e.flow_code = f.flow_code
        WHERE e.value_gwh IS NOT NULL AND e.value_gwh > 0
        GROUP BY e.country_code, e.year
    ),
    region_spatial AS (
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
        WHERE r.level_code IN (0, 2)  -- Support both NUTS0 and NUTS2
    ),
    osm_features AS (
        SELECT 
            region_id,
            snapshot_year as year,
            MAX(CASE WHEN feature_name = 'power_plant_count' THEN feature_value ELSE 0 END)::INTEGER as power_plant_count,
            MAX(CASE WHEN feature_name = 'power_generator_count' THEN feature_value ELSE 0 END)::INTEGER as power_generator_count,
            MAX(CASE WHEN feature_name = 'power_substation_count' THEN feature_value ELSE 0 END)::INTEGER as power_substation_count,
            MAX(CASE WHEN feature_name = 'industrial_area_area_km2' THEN feature_value ELSE 0 END) as industrial_area_km2,
            MAX(CASE WHEN feature_name = 'railway_station_count' THEN feature_value ELSE 0 END)::INTEGER as railway_station_count,
            MAX(CASE WHEN feature_name = 'airport_count' THEN feature_value ELSE 0 END)::INTEGER as airport_count,
            COUNT(*) as osm_feature_count
        FROM fact_geo_region_features
        WHERE snapshot_year = EXTRACT(YEAR FROM CURRENT_DATE)
        GROUP BY region_id, snapshot_year
    )
    SELECT 
        r.region_id,
        r.level_code,
        r.cntr_code,
        r.region_name,
        e.year,
        e.total_energy_gwh,
        e.renewable_energy_gwh,
        e.fossil_energy_gwh,
        e.nuclear_energy_gwh,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.renewable_energy_gwh / e.total_energy_gwh * 100) ELSE 0 END as pct_renewable,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.fossil_energy_gwh / e.total_energy_gwh * 100) ELSE 0 END as pct_fossil,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.nuclear_energy_gwh / e.total_energy_gwh * 100) ELSE 0 END as pct_nuclear,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.production_gwh / e.total_energy_gwh * 100) ELSE 0 END as pct_production,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.imports_gwh / e.total_energy_gwh * 100) ELSE 0 END as pct_imports,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.exports_gwh / e.total_energy_gwh * 100) ELSE 0 END as pct_exports,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.final_consumption_gwh / e.total_energy_gwh * 100) ELSE 0 END as pct_final_consumption,
        r.area_km2,
        r.mount_type,
        r.urbn_type,
        r.coast_type,
        CASE WHEN r.area_km2 > 0 THEN (e.total_energy_gwh / r.area_km2) ELSE NULL END as energy_density_gwh_per_km2,
        CASE WHEN r.area_km2 > 0 THEN (e.renewable_energy_gwh / r.area_km2) ELSE NULL END as renewable_density_gwh_per_km2,
        CASE WHEN r.area_km2 > 0 THEN (e.fossil_energy_gwh / r.area_km2) ELSE NULL END as fossil_density_gwh_per_km2,
        CASE WHEN e.fossil_energy_gwh > 0 THEN (e.renewable_energy_gwh / e.fossil_energy_gwh) ELSE NULL END as renewable_to_fossil_ratio,
        CASE WHEN e.final_consumption_gwh > 0 THEN (e.production_gwh / e.final_consumption_gwh) ELSE NULL END as production_to_consumption_ratio,
        CASE WHEN e.production_gwh > 0 THEN (e.imports_gwh / e.production_gwh) ELSE NULL END as imports_to_production_ratio,
        CASE WHEN r.coast_type IS NOT NULL AND r.coast_type != 'NO_COAST' THEN TRUE ELSE FALSE END as is_coastal,
        CASE WHEN r.mount_type IS NOT NULL AND r.mount_type != 'FLAT' THEN TRUE ELSE FALSE END as is_mountainous,
        CASE WHEN r.urbn_type IS NOT NULL AND (r.urbn_type LIKE '%URBAN%' OR r.urbn_type LIKE '%CITY%') THEN TRUE ELSE FALSE END as is_urban,
        CASE WHEN r.urbn_type IS NOT NULL AND (r.urbn_type LIKE '%RURAL%' OR r.urbn_type LIKE '%REMOTE%') THEN TRUE ELSE FALSE END as is_rural,
        COALESCE(o.power_plant_count, 0) as power_plant_count,
        COALESCE(o.power_generator_count, 0) as power_generator_count,
        COALESCE(o.power_substation_count, 0) as power_substation_count,
        COALESCE(o.industrial_area_km2, 0) as industrial_area_km2,
        COALESCE(o.railway_station_count, 0) as railway_station_count,
        COALESCE(o.airport_count, 0) as airport_count,
        CASE WHEN r.area_km2 > 0 THEN (COALESCE(o.power_plant_count, 0)::NUMERIC / r.area_km2) ELSE NULL END as power_plant_density_per_km2,
        CASE WHEN r.area_km2 > 0 THEN (COALESCE(o.power_generator_count, 0)::NUMERIC / r.area_km2) ELSE NULL END as power_generator_density_per_km2,
        CASE WHEN r.area_km2 > 0 THEN (COALESCE(o.power_substation_count, 0)::NUMERIC / r.area_km2) ELSE NULL END as power_substation_density_per_km2,
        CASE WHEN r.area_km2 > 0 THEN (COALESCE(o.industrial_area_km2, 0) / r.area_km2 * 100) ELSE NULL END as industrial_area_pct,
        e.feature_count,
        COALESCE(o.osm_feature_count, 0) as osm_feature_count
    FROM energy_aggregated e
    INNER JOIN region_spatial r ON e.country_code = r.cntr_code
    LEFT JOIN osm_features o ON r.region_id = o.region_id AND e.year = o.year
    ORDER BY r.region_id, e.year
    """
    
    result = db_hook.get_records(extract_sql)
    
    extract_stats = {
        'total_records': len(result),
        'unique_regions': len(set(row[0] for row in result)) if result else 0,
        'unique_years': len(set(row[4] for row in result)) if result else 0,
    }
    
    logging.info(f"Extracted {extract_stats['total_records']} geo-enrichment feature records")
    logging.info(f"Unique regions: {extract_stats['unique_regions']}, years: {extract_stats['unique_years']}")
    
    return extract_stats


def load_geo_enrichment_dataset(**context):
    """
    Load geo-enrichment features into ml_dataset_geo_enrichment_v1 table.
    """
    ti = context['ti']
    extract_stats = ti.xcom_pull(task_ids='extract_geo_enrichment_features')
    
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    logging.info("Loading geo-enrichment features into ml_dataset_geo_enrichment_v1...")
    
    # Load features
    insert_sql = """
    WITH energy_aggregated AS (
        SELECT 
            e.country_code,
            e.year,
            SUM(e.value_gwh) as total_energy_gwh,
            SUM(CASE WHEN p.renewable_flag = TRUE THEN e.value_gwh ELSE 0 END) as renewable_energy_gwh,
            SUM(CASE WHEN p.fossil_flag = TRUE THEN e.value_gwh ELSE 0 END) as fossil_energy_gwh,
            SUM(CASE WHEN p.product_code LIKE '%NUCLEAR%' OR p.product_code = 'N900H' THEN e.value_gwh ELSE 0 END) as nuclear_energy_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%PRODUCTION%' OR f.flow_code = 'P1000' THEN e.value_gwh ELSE 0 END) as production_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%IMPORT%' OR f.flow_code = 'IMP' THEN e.value_gwh ELSE 0 END) as imports_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%EXPORT%' OR f.flow_code = 'EXP' THEN e.value_gwh ELSE 0 END) as exports_gwh,
            SUM(CASE WHEN f.flow_code LIKE '%CONSUMPTION%' OR f.flow_code LIKE '%FC%' THEN e.value_gwh ELSE 0 END) as final_consumption_gwh,
            COUNT(*) as feature_count
        FROM fact_energy_annual e
        LEFT JOIN dim_energy_product p ON e.product_code = p.product_code
        LEFT JOIN dim_energy_flow f ON e.flow_code = f.flow_code
        WHERE e.value_gwh IS NOT NULL AND e.value_gwh > 0
        GROUP BY e.country_code, e.year
    ),
    region_spatial AS (
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
    INSERT INTO ml_dataset_geo_enrichment_v1 (
        region_id, level_code, cntr_code, region_name, year,
        total_energy_gwh, renewable_energy_gwh, fossil_energy_gwh, nuclear_energy_gwh,
        pct_renewable, pct_fossil, pct_nuclear,
        pct_production, pct_imports, pct_exports, pct_final_consumption,
        area_km2, mount_type, urbn_type, coast_type,
        energy_density_gwh_per_km2, renewable_density_gwh_per_km2, fossil_density_gwh_per_km2,
        renewable_to_fossil_ratio, production_to_consumption_ratio, imports_to_production_ratio,
        is_coastal, is_mountainous, is_urban, is_rural,
        power_plant_count, power_generator_count, power_substation_count,
        industrial_area_km2, railway_station_count, airport_count,
        power_plant_density_per_km2, power_generator_density_per_km2, power_substation_density_per_km2,
        industrial_area_pct,
        feature_count, osm_feature_count, updated_at
    )
    SELECT 
        r.region_id,
        r.level_code,
        r.cntr_code,
        r.region_name,
        e.year,
        e.total_energy_gwh,
        e.renewable_energy_gwh,
        e.fossil_energy_gwh,
        e.nuclear_energy_gwh,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.renewable_energy_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.fossil_energy_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.nuclear_energy_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.production_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.imports_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.exports_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        CASE WHEN e.total_energy_gwh > 0 THEN (e.final_consumption_gwh / e.total_energy_gwh * 100) ELSE 0 END,
        r.area_km2,
        r.mount_type,
        r.urbn_type,
        r.coast_type,
        CASE WHEN r.area_km2 > 0 THEN (e.total_energy_gwh / r.area_km2) ELSE NULL END,
        CASE WHEN r.area_km2 > 0 THEN (e.renewable_energy_gwh / r.area_km2) ELSE NULL END,
        CASE WHEN r.area_km2 > 0 THEN (e.fossil_energy_gwh / r.area_km2) ELSE NULL END,
        CASE WHEN e.fossil_energy_gwh > 0 THEN (e.renewable_energy_gwh / e.fossil_energy_gwh) ELSE NULL END,
        CASE WHEN e.final_consumption_gwh > 0 THEN (e.production_gwh / e.final_consumption_gwh) ELSE NULL END,
        CASE WHEN e.production_gwh > 0 THEN (e.imports_gwh / e.production_gwh) ELSE NULL END,
        CASE WHEN r.coast_type IS NOT NULL AND r.coast_type != 'NO_COAST' THEN TRUE ELSE FALSE END,
        CASE WHEN r.mount_type IS NOT NULL AND r.mount_type != 'FLAT' THEN TRUE ELSE FALSE END,
        CASE WHEN r.urbn_type IS NOT NULL AND (r.urbn_type LIKE '%URBAN%' OR r.urbn_type LIKE '%CITY%') THEN TRUE ELSE FALSE END,
        CASE WHEN r.urbn_type IS NOT NULL AND (r.urbn_type LIKE '%RURAL%' OR r.urbn_type LIKE '%REMOTE%') THEN TRUE ELSE FALSE END,
        COALESCE(o.power_plant_count, 0),
        COALESCE(o.power_generator_count, 0),
        COALESCE(o.power_substation_count, 0),
        COALESCE(o.industrial_area_km2, 0),
        COALESCE(o.railway_station_count, 0),
        COALESCE(o.airport_count, 0),
        CASE WHEN r.area_km2 > 0 THEN (COALESCE(o.power_plant_count, 0)::NUMERIC / r.area_km2) ELSE NULL END,
        CASE WHEN r.area_km2 > 0 THEN (COALESCE(o.power_generator_count, 0)::NUMERIC / r.area_km2) ELSE NULL END,
        CASE WHEN r.area_km2 > 0 THEN (COALESCE(o.power_substation_count, 0)::NUMERIC / r.area_km2) ELSE NULL END,
        CASE WHEN r.area_km2 > 0 THEN (COALESCE(o.industrial_area_km2, 0) / r.area_km2 * 100) ELSE NULL END,
        e.feature_count,
        COALESCE(o.osm_feature_count, 0),
        NOW()
    FROM energy_aggregated e
    INNER JOIN region_spatial r ON e.country_code = r.cntr_code
    LEFT JOIN osm_features o ON r.region_id = o.region_id AND e.year = o.year
    ON CONFLICT (region_id, year) 
    DO UPDATE SET
        total_energy_gwh = EXCLUDED.total_energy_gwh,
        renewable_energy_gwh = EXCLUDED.renewable_energy_gwh,
        fossil_energy_gwh = EXCLUDED.fossil_energy_gwh,
        nuclear_energy_gwh = EXCLUDED.nuclear_energy_gwh,
        pct_renewable = EXCLUDED.pct_renewable,
        pct_fossil = EXCLUDED.pct_fossil,
        pct_nuclear = EXCLUDED.pct_nuclear,
        pct_production = EXCLUDED.pct_production,
        pct_imports = EXCLUDED.pct_imports,
        pct_exports = EXCLUDED.pct_exports,
        pct_final_consumption = EXCLUDED.pct_final_consumption,
        area_km2 = EXCLUDED.area_km2,
        mount_type = EXCLUDED.mount_type,
        urbn_type = EXCLUDED.urbn_type,
        coast_type = EXCLUDED.coast_type,
        energy_density_gwh_per_km2 = EXCLUDED.energy_density_gwh_per_km2,
        renewable_density_gwh_per_km2 = EXCLUDED.renewable_density_gwh_per_km2,
        fossil_density_gwh_per_km2 = EXCLUDED.fossil_density_gwh_per_km2,
        renewable_to_fossil_ratio = EXCLUDED.renewable_to_fossil_ratio,
        production_to_consumption_ratio = EXCLUDED.production_to_consumption_ratio,
        imports_to_production_ratio = EXCLUDED.imports_to_production_ratio,
        is_coastal = EXCLUDED.is_coastal,
        is_mountainous = EXCLUDED.is_mountainous,
        is_urban = EXCLUDED.is_urban,
        is_rural = EXCLUDED.is_rural,
        power_plant_count = EXCLUDED.power_plant_count,
        power_generator_count = EXCLUDED.power_generator_count,
        power_substation_count = EXCLUDED.power_substation_count,
        industrial_area_km2 = EXCLUDED.industrial_area_km2,
        railway_station_count = EXCLUDED.railway_station_count,
        airport_count = EXCLUDED.airport_count,
        power_plant_density_per_km2 = EXCLUDED.power_plant_density_per_km2,
        power_generator_density_per_km2 = EXCLUDED.power_generator_density_per_km2,
        power_substation_density_per_km2 = EXCLUDED.power_substation_density_per_km2,
        industrial_area_pct = EXCLUDED.industrial_area_pct,
        feature_count = EXCLUDED.feature_count,
        osm_feature_count = EXCLUDED.osm_feature_count,
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
            AVG(energy_density_gwh_per_km2) as avg_density
        FROM ml_dataset_geo_enrichment_v1
    """
    stats = db_hook.get_first(stats_sql)
    
    load_stats = {
        'total_records': stats[0],
        'unique_regions': stats[1],
        'unique_years': stats[2],
        'min_year': stats[3],
        'max_year': stats[4],
        'avg_energy_gwh': float(stats[5]) if stats[5] else 0,
        'avg_density': float(stats[6]) if stats[6] else 0,
    }
    
    logging.info(f"Dataset loaded: {load_stats['total_records']} records, "
                 f"{load_stats['unique_regions']} regions, "
                 f"{load_stats['unique_years']} years")
    
    return load_stats


def validate_geo_enrichment_dataset(**context):
    """
    Validate the geo-enrichment dataset.
    """
    ti = context['ti']
    load_stats = ti.xcom_pull(task_ids='load_geo_enrichment_dataset')
    
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    logging.info("Validating geo-enrichment dataset...")
    
    validation_queries = {
        'null_checks': """
            SELECT 
                COUNT(*) as total,
                SUM(CASE WHEN total_energy_gwh IS NULL THEN 1 ELSE 0 END) as null_energy,
                SUM(CASE WHEN area_km2 IS NULL THEN 1 ELSE 0 END) as null_area,
                SUM(CASE WHEN energy_density_gwh_per_km2 IS NULL THEN 1 ELSE 0 END) as null_density
            FROM ml_dataset_geo_enrichment_v1
        """,
        'spatial_feature_checks': """
            SELECT 
                COUNT(*) as total,
                SUM(CASE WHEN is_coastal IS NULL THEN 1 ELSE 0 END) as null_coastal,
                SUM(CASE WHEN is_mountainous IS NULL THEN 1 ELSE 0 END) as null_mountainous,
                SUM(CASE WHEN is_urban IS NULL THEN 1 ELSE 0 END) as null_urban
            FROM ml_dataset_geo_enrichment_v1
        """,
        'feature_completeness': """
            SELECT 
                COUNT(*) as total,
                AVG(CASE WHEN total_energy_gwh > 0 THEN 1.0 ELSE 0.0 END) * 100 as pct_with_energy,
                AVG(CASE WHEN area_km2 > 0 THEN 1.0 ELSE 0.0 END) * 100 as pct_with_area,
                AVG(CASE WHEN energy_density_gwh_per_km2 IS NOT NULL THEN 1.0 ELSE 0.0 END) * 100 as pct_with_density
            FROM ml_dataset_geo_enrichment_v1
        """,
    }
    
    validation_results = {}
    
    # Null checks
    null_result = db_hook.get_first(validation_queries['null_checks'])
    validation_results['null_checks'] = {
        'total': null_result[0],
        'null_energy': null_result[1],
        'null_area': null_result[2],
        'null_density': null_result[3]
    }
    
    # Spatial feature checks
    spatial_result = db_hook.get_first(validation_queries['spatial_feature_checks'])
    validation_results['spatial_feature_checks'] = {
        'total': spatial_result[0],
        'null_coastal': spatial_result[1],
        'null_mountainous': spatial_result[2],
        'null_urban': spatial_result[3]
    }
    
    # Feature completeness
    completeness_result = db_hook.get_first(validation_queries['feature_completeness'])
    validation_results['feature_completeness'] = {
        'total': completeness_result[0],
        'pct_with_energy': float(completeness_result[1]) if completeness_result[1] else 0,
        'pct_with_area': float(completeness_result[2]) if completeness_result[2] else 0,
        'pct_with_density': float(completeness_result[3]) if completeness_result[3] else 0
    }
    
    # Check for validation issues
    issues = []
    if validation_results['null_checks']['null_energy'] > 0:
        issues.append(f"Found {validation_results['null_checks']['null_energy']} records with null energy")
    if validation_results['null_checks']['null_area'] > 0:
        issues.append(f"Found {validation_results['null_checks']['null_area']} records with null area")
    
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
    load_stats = ti.xcom_pull(task_ids='load_geo_enrichment_dataset')
    validation_results = ti.xcom_pull(task_ids='validate_geo_enrichment_dataset')
    
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
        'ml_dataset_geo_enrichment_v1',
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
        'avg_density': load_stats['avg_density'],
        'validation_passed': validation_results['is_valid'],
        'validation_issues': validation_results['issues']
    }
    
    import json
    status = 'success' if validation_results['is_valid'] else 'partial'
    db_hook.run(log_sql, parameters=(
        load_stats['total_records'],
        status,
        json.dumps(metadata)
    ))
    
    logging.info("✅ Dataset creation logged to metadata table")
    
    return {'logged': True}


# Task definitions
create_table_task = PythonOperator(
    task_id='create_geo_enrichment_dataset_table',
    python_callable=create_geo_enrichment_dataset_table,
    dag=dag,
)

extract_features_task = PythonOperator(
    task_id='extract_geo_enrichment_features',
    python_callable=extract_geo_enrichment_features,
    dag=dag,
)

load_dataset_task = PythonOperator(
    task_id='load_geo_enrichment_dataset',
    python_callable=load_geo_enrichment_dataset,
    dag=dag,
)

validate_dataset_task = PythonOperator(
    task_id='validate_geo_enrichment_dataset',
    python_callable=validate_geo_enrichment_dataset,
    dag=dag,
)

log_creation_task = PythonOperator(
    task_id='log_dataset_creation',
    python_callable=log_dataset_creation,
    dag=dag,
)

# Task dependencies
create_table_task >> extract_features_task >> load_dataset_task >> validate_dataset_task >> log_creation_task
