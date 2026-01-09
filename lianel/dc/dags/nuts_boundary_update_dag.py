"""
NUTS Boundary Update DAG

This DAG downloads NUTS boundaries from Eurostat GISCO, transforms them to EPSG:3035,
calculates areas, validates geometries, and loads them into dim_region table.

Schedule: Annually (first Sunday of January at 03:00 UTC)
Manual trigger: Yes (for testing or mid-year updates)
"""
from datetime import datetime, timedelta
from airflow import DAG
from airflow.operators.python import PythonOperator
from airflow.providers.postgres.hooks.postgres import PostgresHook
import geopandas as gpd
import requests
from pathlib import Path
import tempfile
import logging

default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'start_date': datetime(2026, 1, 1),
    'email_on_failure': False,
    'email_on_retry': False,
    'retries': 2,
    'retry_delay': timedelta(minutes=10),
    'execution_timeout': timedelta(hours=2),
}

dag = DAG(
    'nuts_boundary_update',
    default_args=default_args,
    description='Download and update NUTS boundaries from Eurostat GISCO',
    schedule='0 3 1-7 * 1',  # First Sunday of January at 03:00 UTC
    catchup=False,
    tags=['geospatial', 'nuts', 'gisco', 'annual'],
    max_active_runs=1,
)

# GISCO base URL and file mappings
GISCO_BASE_URL = "https://gisco-services.ec.europa.eu/distribution/v2/nuts/geojson/"
NUTS_VERSION = "2021"  # Update this when new versions are released
NUTS_FILES = {
    0: f"NUTS_RG_01M_{NUTS_VERSION}_4326_LEVL_0.geojson",
    1: f"NUTS_RG_01M_{NUTS_VERSION}_4326_LEVL_1.geojson",
    2: f"NUTS_RG_01M_{NUTS_VERSION}_4326_LEVL_2.geojson",
}


def download_nuts_file(level: int, **context) -> str:
    """
    Download NUTS GeoJSON file from GISCO.
    
    Args:
        level: NUTS level (0, 1, or 2)
    
    Returns:
        Path to downloaded file
    """
    filename = NUTS_FILES[level]
    url = GISCO_BASE_URL + filename
    
    logging.info(f"Downloading NUTS level {level} from {url}")
    
    # Create temporary directory
    temp_dir = Path(tempfile.gettempdir()) / "nuts_download"
    temp_dir.mkdir(exist_ok=True)
    filepath = temp_dir / filename
    
    # Download file
    response = requests.get(url, stream=True, timeout=300)
    response.raise_for_status()
    
    with open(filepath, 'wb') as f:
        for chunk in response.iter_content(chunk_size=8192):
            f.write(chunk)
    
    file_size_mb = filepath.stat().st_size / (1024 * 1024)
    logging.info(f"Downloaded {filename}: {file_size_mb:.2f} MB")
    
    return str(filepath)


def process_nuts_data(level: int, **context) -> dict:
    """
    Process NUTS GeoJSON: transform to EPSG:3035, calculate area, validate.
    
    Args:
        level: NUTS level (0, 1, or 2)
    
    Returns:
        Dictionary with processing statistics
    """
    ti = context['ti']
    filepath = ti.xcom_pull(task_ids=f'download_nuts_level_{level}')
    
    logging.info(f"Processing NUTS level {level} from {filepath}")
    
    # Load GeoJSON
    gdf = gpd.read_file(filepath)
    logging.info(f"Loaded {len(gdf)} features for NUTS level {level}")
    
    # Validate source CRS (should be EPSG:4326)
    if gdf.crs is None:
        logging.warning("No CRS found, assuming EPSG:4326")
        gdf.set_crs("EPSG:4326", inplace=True)
    elif gdf.crs.to_string() != "EPSG:4326":
        logging.warning(f"Unexpected CRS: {gdf.crs}, transforming from EPSG:4326")
        gdf.set_crs("EPSG:4326", inplace=True)
    
    # Keep original geometry for WGS84
    gdf['geometry_wgs84'] = gdf.geometry
    
    # Transform to EPSG:3035 for analysis
    gdf_proj = gdf.to_crs("EPSG:3035")
    
    # Calculate area in km²
    gdf_proj['area_km2'] = gdf_proj.geometry.area / 1_000_000
    
    # Validate geometries
    invalid_count = (~gdf_proj.geometry.is_valid).sum()
    if invalid_count > 0:
        logging.warning(f"Found {invalid_count} invalid geometries, attempting to fix...")
        gdf_proj.geometry = gdf_proj.geometry.buffer(0)  # Fix invalid geometries
    
    # Rename columns to match database schema
    column_mapping = {
        'NUTS_ID': 'region_id',
        'LEVL_CODE': 'level_code',
        'CNTR_CODE': 'cntr_code',
        'NAME_LATN': 'name_latn',
        'NUTS_NAME': 'nuts_name',
        'MOUNT_TYPE': 'mount_type',
        'URBN_TYPE': 'urbn_type',
        'COAST_TYPE': 'coast_type',
    }
    
    # Select and rename columns
    columns_to_keep = list(column_mapping.keys()) + ['geometry', 'geometry_wgs84', 'area_km2']
    gdf_final = gdf_proj[[col for col in columns_to_keep if col in gdf_proj.columns]].copy()
    gdf_final.rename(columns=column_mapping, inplace=True)
    
    # Ensure level_code matches
    gdf_final['level_code'] = level
    
    # Store processed data in XCom (as GeoDataFrame serialization)
    # We'll save to a temporary file and pass the path
    temp_dir = Path(tempfile.gettempdir()) / "nuts_processed"
    temp_dir.mkdir(exist_ok=True)
    processed_file = temp_dir / f"nuts_level_{level}_processed.geojson"
    gdf_final.to_file(processed_file, driver='GeoJSON')
    
    stats = {
        'level': level,
        'feature_count': len(gdf_final),
        'min_area_km2': float(gdf_final['area_km2'].min()),
        'max_area_km2': float(gdf_final['area_km2'].max()),
        'mean_area_km2': float(gdf_final['area_km2'].mean()),
        'total_area_km2': float(gdf_final['area_km2'].sum()),
        'processed_file': str(processed_file),
    }
    
    logging.info(f"NUTS level {level} processed: {stats['feature_count']} features")
    logging.info(f"Area range: {stats['min_area_km2']:.2f} - {stats['max_area_km2']:.2f} km²")
    
    return stats


def load_nuts_to_database(level: int, **context) -> dict:
    """
    Load processed NUTS data into dim_region table.
    
    Args:
        level: NUTS level (0, 1, or 2)
    
    Returns:
        Dictionary with load statistics
    """
    ti = context['ti']
    stats = ti.xcom_pull(task_ids=f'process_nuts_level_{level}')
    processed_file = stats['processed_file']
    
    logging.info(f"Loading NUTS level {level} from {processed_file}")
    
    # Load processed GeoDataFrame
    gdf = gpd.read_file(processed_file)
    
    # Get database connection
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    engine = db_hook.get_sqlalchemy_engine()
    
    # Prepare data for insertion
    # Convert geometry columns to WKT for PostGIS
    gdf['geometry_wkt_3035'] = gdf['geometry'].apply(lambda x: x.wkt if x else None)
    gdf['geometry_wkt_4326'] = gdf['geometry_wgs84'].apply(lambda x: x.wkt if x else None)
    
    # Select columns for database
    db_columns = [
        'region_id', 'level_code', 'cntr_code', 'name_latn', 'nuts_name',
        'mount_type', 'urbn_type', 'coast_type', 'area_km2'
    ]
    
    # Insert/update records
    inserted = 0
    updated = 0
    errors = 0
    
    for _, row in gdf.iterrows():
        try:
            # Check if region exists
            check_sql = """
                SELECT region_id FROM dim_region 
                WHERE region_id = %s
            """
            existing = db_hook.get_first(check_sql, parameters=(row['region_id'],))
            
            if existing:
                # Update existing record
                update_sql = """
                    UPDATE dim_region SET
                        level_code = %s,
                        cntr_code = %s,
                        name_latn = %s,
                        nuts_name = %s,
                        mount_type = %s,
                        urbn_type = %s,
                        coast_type = %s,
                        area_km2 = %s,
                        geometry = ST_GeomFromText(%s, 3035),
                        geometry_wgs84 = ST_GeomFromText(%s, 4326),
                        updated_at = NOW()
                    WHERE region_id = %s
                """
                db_hook.run(update_sql, parameters=(
                    int(row['level_code']),
                    row['cntr_code'],
                    row.get('name_latn'),
                    row.get('nuts_name'),
                    row.get('mount_type'),
                    row.get('urbn_type'),
                    row.get('coast_type'),
                    float(row['area_km2']) if row['area_km2'] else None,
                    row['geometry_wkt_3035'],
                    row['geometry_wkt_4326'],
                    row['region_id'],
                ))
                updated += 1
            else:
                # Insert new record
                insert_sql = """
                    INSERT INTO dim_region (
                        region_id, level_code, cntr_code, name_latn, nuts_name,
                        mount_type, urbn_type, coast_type, area_km2,
                        geometry, geometry_wgs84, created_at, updated_at
                    ) VALUES (
                        %s, %s, %s, %s, %s, %s, %s, %s, %s,
                        ST_GeomFromText(%s, 3035),
                        ST_GeomFromText(%s, 4326),
                        NOW(), NOW()
                    )
                """
                db_hook.run(insert_sql, parameters=(
                    row['region_id'],
                    int(row['level_code']),
                    row['cntr_code'],
                    row.get('name_latn'),
                    row.get('nuts_name'),
                    row.get('mount_type'),
                    row.get('urbn_type'),
                    row.get('coast_type'),
                    float(row['area_km2']) if row['area_km2'] else None,
                    row['geometry_wkt_3035'],
                    row['geometry_wkt_4326'],
                ))
                inserted += 1
        except Exception as e:
            logging.error(f"Error processing region {row.get('region_id', 'unknown')}: {str(e)}")
            errors += 1
    
    # Rebuild spatial indexes
    logging.info("Rebuilding spatial indexes...")
    db_hook.run("REINDEX INDEX idx_region_geometry;")
    db_hook.run("REINDEX INDEX idx_region_geometry_wgs84;")
    
    load_stats = {
        'level': level,
        'inserted': inserted,
        'updated': updated,
        'errors': errors,
        'total': inserted + updated,
    }
    
    logging.info(f"NUTS level {level} loaded: {load_stats['inserted']} inserted, "
                 f"{load_stats['updated']} updated, {load_stats['errors']} errors")
    
    return load_stats


def validate_nuts_data(**context) -> dict:
    """
    Validate loaded NUTS data: check geometry validity, hierarchy, counts.
    
    Returns:
        Dictionary with validation results
    """
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    validation_results = {}
    
    # Check counts by level
    for level in [0, 1, 2]:
        count_sql = f"SELECT COUNT(*) FROM dim_region WHERE level_code = {level}"
        count = db_hook.get_first(count_sql)[0]
        validation_results[f'level_{level}_count'] = count
        logging.info(f"NUTS level {level}: {count} regions")
    
    # Check geometry validity
    validity_sql = """
        SELECT 
            level_code,
            COUNT(*) as total,
            SUM(CASE WHEN ST_IsValid(geometry) THEN 1 ELSE 0 END) as valid,
            SUM(CASE WHEN NOT ST_IsValid(geometry) THEN 1 ELSE 0 END) as invalid
        FROM dim_region
        GROUP BY level_code
        ORDER BY level_code
    """
    validity_results = db_hook.get_records(validity_sql)
    
    validation_results['geometry_validity'] = {}
    for row in validity_results:
        level, total, valid, invalid = row
        validation_results['geometry_validity'][f'level_{level}'] = {
            'total': total,
            'valid': valid,
            'invalid': invalid,
        }
        if invalid > 0:
            logging.warning(f"NUTS level {level}: {invalid} invalid geometries found")
    
    # Check country code consistency
    consistency_sql = """
        SELECT 
            COUNT(*) as total,
            SUM(CASE WHEN LEFT(region_id, 2) = cntr_code THEN 1 ELSE 0 END) as consistent
        FROM dim_region
        WHERE level_code > 0
    """
    consistency_result = db_hook.get_first(consistency_sql)
    total, consistent = consistency_result
    validation_results['country_code_consistency'] = {
        'total': total,
        'consistent': consistent,
        'inconsistent': total - consistent,
    }
    
    if total - consistent > 0:
        logging.warning(f"Found {total - consistent} regions with inconsistent country codes")
    
    logging.info("NUTS data validation complete")
    return validation_results


def log_update_summary(**context) -> None:
    """Log update summary to metadata table"""
    ti = context['ti']
    
    # Collect statistics from all levels
    all_stats = []
    for level in [0, 1, 2]:
        process_stats = ti.xcom_pull(task_ids=f'process_nuts_level_{level}')
        load_stats = ti.xcom_pull(task_ids=f'load_nuts_level_{level}')
        if process_stats and load_stats:
            all_stats.append({
                'level': level,
                **process_stats,
                **load_stats,
            })
    
    validation_results = ti.xcom_pull(task_ids='validate_nuts_data')
    
    # Log to metadata table
    db_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    summary = {
        'source_system': 'gisco',
        'update_type': 'nuts_boundaries',
        'nuts_version': NUTS_VERSION,
        'levels_processed': [s['level'] for s in all_stats],
        'total_regions': sum(s.get('inserted', 0) + s.get('updated', 0) for s in all_stats),
        'validation': validation_results,
    }
    
    insert_sql = """
        INSERT INTO meta_ingestion_log (
            source_system, status, records_inserted, records_updated,
            started_at, completed_at, metadata
        ) VALUES (
            %s, %s, %s, %s, %s, %s, %s::jsonb
        )
    """
    
    total_inserted = sum(s.get('inserted', 0) for s in all_stats)
    total_updated = sum(s.get('updated', 0) for s in all_stats)
    
    import json
    db_hook.run(insert_sql, parameters=(
        'gisco',
        'success' if validation_results else 'partial',
        total_inserted,
        total_updated,
        context['execution_date'],
        datetime.now(),
        json.dumps(summary),
    ))
    
    logging.info(f"NUTS boundary update complete: {total_inserted} inserted, {total_updated} updated")


# Create tasks for each NUTS level
for level in [0, 1, 2]:
    download_task = PythonOperator(
        task_id=f'download_nuts_level_{level}',
        python_callable=download_nuts_file,
        op_kwargs={'level': level},
        dag=dag,
    )
    
    process_task = PythonOperator(
        task_id=f'process_nuts_level_{level}',
        python_callable=process_nuts_data,
        op_kwargs={'level': level},
        dag=dag,
    )
    
    load_task = PythonOperator(
        task_id=f'load_nuts_level_{level}',
        python_callable=load_nuts_to_database,
        op_kwargs={'level': level},
        dag=dag,
    )
    
    # Set task dependencies
    download_task >> process_task >> load_task

# Validation and logging tasks
validate_task = PythonOperator(
    task_id='validate_nuts_data',
    python_callable=validate_nuts_data,
    dag=dag,
)

log_task = PythonOperator(
    task_id='log_update_summary',
    python_callable=log_update_summary,
    dag=dag,
)

# Set final dependencies: all load tasks -> validate -> log
for level in [0, 1, 2]:
    load_task = dag.get_task(f'load_nuts_level_{level}')
    load_task >> validate_task

validate_task >> log_task
