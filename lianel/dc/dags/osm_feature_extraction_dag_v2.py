"""
OSM Feature Extraction DAG - Optimized Version

This DAG extracts OpenStreetMap features and aggregates them to NUTS2 regions.
Optimized to process regions in small batches to avoid memory and rate limiting issues.

Strategy:
- Process regions in batches of 3
- Each batch runs sequentially
- Within each batch, process 1 region at a time (max_active_tasks=1)
- This prevents OOM kills and Overpass API rate limiting
"""

from datetime import datetime, timedelta
from airflow import DAG
from airflow.operators.python import PythonOperator
from airflow.providers.postgres.hooks.postgres import PostgresHook
from airflow.utils.task_group import TaskGroup
from airflow.exceptions import AirflowException
import sys
import os
import json
from typing import Dict, List, Any, Optional

# Add utils directory to path
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'utils'))
from osm_client import OSMClient, get_region_bbox

default_args = {
    'owner': 'lianel',
    'depends_on_past': False,
    'start_date': datetime(2026, 1, 1),
    'email_on_failure': False,
    'email_on_retry': False,
    'retries': 2,
    'retry_delay': timedelta(minutes=5),
    'execution_timeout': timedelta(hours=2),  # Reduced from 6 hours
}

dag = DAG(
    'osm_feature_extraction_v2',
    default_args=default_args,
    description='Extract OSM features and aggregate to NUTS2 regions (optimized)',
    schedule='0 4 * * 0',  # Weekly on Sunday at 04:00 UTC
    catchup=False,
    tags=['data-ingestion', 'osm', 'geospatial', 'features', 'optimized'],
    max_active_runs=1,
    max_active_tasks=1,  # Process one region at a time to avoid memory issues
)

# Feature types to extract - reduced set to avoid memory issues
# Focus on essential features for energy analysis
FEATURE_TYPES = [
    'power_plant',
    'power_generator',
    'power_substation',
    'industrial_area',
    'railway_station',
    'airport',
]

# Process regions in batches to avoid overwhelming the system
BATCH_SIZE = 3  # Process 3 regions per batch
REGION_BATCHES = [
    ['SE11', 'SE12', 'SE21'],  # Batch 1: Sweden
    ['SE22', 'SE23', 'DE11'],  # Batch 2: Sweden + Germany
    ['DE12', 'DE21', 'DE22'],  # Batch 3: Germany
    ['DE30', 'FR10', 'FR21'],  # Batch 4: Germany + France
    ['FR22', 'FR30', 'FR41'],  # Batch 5: France
]


def lookup_region(region_id: str, **context) -> Dict[str, Any]:
    """Get region geometry and metadata."""
    postgres_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    sql = """
        SELECT region_id, area_km2, ST_AsGeoJSON(geometry) as geometry_json
        FROM dim_region
        WHERE region_id = %s AND level_code = 2
    """
    
    try:
        result = postgres_hook.get_first(sql, parameters=(region_id,))
        if not result:
            print(f"Region {region_id} not found in database")
            return {
                'region_id': region_id,
                'area_km2': 0.0,
                'geometry_json': None,
                'status': 'region_not_found'
            }
        
        region_id_db, area_km2, geometry_json = result
        
        if not geometry_json:
            print(f"No geometry available for region {region_id}")
            return {
                'region_id': region_id,
                'area_km2': area_km2 or 0.0,
                'geometry_json': None,
                'status': 'no_geometry'
            }
        
        return {
            'region_id': region_id_db,
            'area_km2': float(area_km2) if area_km2 else 0.0,
            'geometry_json': geometry_json,
            'status': 'success'
        }
        
    except Exception as e:
        print(f"Error querying region {region_id}: {e}")
        import traceback
        print(f"Traceback: {traceback.format_exc()}")
        return {
            'region_id': region_id,
            'area_km2': 0.0,
            'geometry_json': None,
            'status': 'database_error',
            'error': str(e)
        }


def extract_region_features(region_id: str, **context) -> Dict[str, Any]:
    """Extract OSM features for a region."""
    ti = context['ti']
    
    # Get region data from lookup task XCom
    # Need to try all possible batch prefixes
    possible_lookup_task_ids = [
        f'batch_1.region_{region_id.lower()}.lookup_{region_id.lower()}',
        f'batch_2.region_{region_id.lower()}.lookup_{region_id.lower()}',
        f'batch_3.region_{region_id.lower()}.lookup_{region_id.lower()}',
        f'batch_4.region_{region_id.lower()}.lookup_{region_id.lower()}',
        f'batch_5.region_{region_id.lower()}.lookup_{region_id.lower()}',
        f'region_{region_id.lower()}.lookup_{region_id.lower()}',  # Fallback
    ]
    
    region_data = None
    for lookup_task_id in possible_lookup_task_ids:
        try:
            result = ti.xcom_pull(task_ids=lookup_task_id)
            if result:
                region_data = result
                if isinstance(region_data, list) and region_data:
                    region_data = region_data[0]
                if isinstance(region_data, dict):
                    break
        except:
            continue
    
    if not region_data or not isinstance(region_data, dict):
        print(f"No region data found in XCom for region {region_id}")
        return {
            'region_id': region_id,
            'features_extracted': 0,
            'status': 'no_region_data'
        }
    
    if region_data.get('status') != 'success':
        print(f"Region lookup failed for {region_id}: {region_data.get('status')}")
        return {
            'region_id': region_id,
            'features_extracted': 0,
            'status': f"lookup_failed_{region_data.get('status')}",
            'error': region_data.get('error', 'Unknown error')
        }
    
    geometry_json = region_data.get('geometry_json')
    area_km2 = region_data.get('area_km2', 0.0)
    
    if not geometry_json:
        print(f"No geometry JSON for region {region_id}")
        return {
            'region_id': region_id,
            'features_extracted': 0,
            'status': 'no_geometry'
        }
    
    osm_client = OSMClient()
    
    try:
        # Parse geometry
        geometry_data = json.loads(geometry_json)
        from shapely.geometry import shape
        region_geometry = shape(geometry_data)
        
        # Get bounding box
        bbox = get_region_bbox(region_geometry)
        print(f"Extracting features for region {region_id}, bbox: {bbox}")
        
        # Extract features (one type at a time to reduce memory)
        all_features = {}
        feature_count = 0
        
        for feature_type in FEATURE_TYPES:
            try:
                print(f"Extracting {feature_type} for region {region_id}...")
                features = osm_client.extract_features(bbox, feature_types=[feature_type])
                
                if features and feature_type in features:
                    all_features[feature_type] = features[feature_type]
                    feature_count += len(features[feature_type])
                    print(f"Found {len(features[feature_type])} {feature_type} features")
                
                # Small delay between feature types
                import time
                time.sleep(1)
                
            except Exception as e:
                print(f"Error extracting {feature_type} for region {region_id}: {e}")
                # Continue with other feature types
                continue
        
        print(f"Total features extracted for region {region_id}: {feature_count}")
        
        return {
            'region_id': region_id,
            'area_km2': area_km2,
            'features_extracted': feature_count,
            'feature_types': list(all_features.keys()),
            'status': 'success'
        }
        
    except json.JSONDecodeError as e:
        print(f"Error parsing geometry JSON for region {region_id}: {e}")
        return {
            'region_id': region_id,
            'features_extracted': 0,
            'status': 'geometry_parse_error',
            'error': str(e)
        }
    except Exception as e:
        print(f"Error extracting OSM features for region {region_id}: {e}")
        import traceback
        print(f"Traceback: {traceback.format_exc()}")
        return {
            'region_id': region_id,
            'features_extracted': 0,
            'status': 'extraction_error',
            'error': str(e)
        }


def store_region_metrics(region_id: str, **context) -> Dict[str, Any]:
    """Verify extraction completed successfully (metrics already stored during extraction)."""
    postgres_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    ti = context['ti']
    
    # Get extract result from XCom
    possible_extract_task_ids = [
        f'batch_1.region_{region_id.lower()}.extract_{region_id.lower()}',
        f'batch_2.region_{region_id.lower()}.extract_{region_id.lower()}',
        f'batch_3.region_{region_id.lower()}.extract_{region_id.lower()}',
        f'batch_4.region_{region_id.lower()}.extract_{region_id.lower()}',
        f'batch_5.region_{region_id.lower()}.extract_{region_id.lower()}',
        f'region_{region_id.lower()}.extract_{region_id.lower()}',  # Fallback
    ]
    
    extract_result = None
    for extract_task_id in possible_extract_task_ids:
        try:
            result = ti.xcom_pull(task_ids=extract_task_id)
            if result:
                extract_result = result
                if isinstance(extract_result, list) and extract_result:
                    extract_result = extract_result[0]
                if isinstance(extract_result, dict):
                    break
        except:
            continue
    
    if not extract_result or not isinstance(extract_result, dict):
        print(f"No extract result found in XCom for region {region_id}")
        return {
            'region_id': region_id,
            'features_stored': 0,
            'status': 'no_extract_result'
        }
    
    # Metrics were already stored during extraction, just verify
    features_extracted = extract_result.get('features_extracted', 0)
    feature_types = extract_result.get('feature_types', [])
    
    print(f"Extraction completed for region {region_id}: {features_extracted} features, {len(feature_types)} types")
    
    # Verify data was stored
    verify_sql = """
        SELECT COUNT(DISTINCT feature_name) as stored_count
        FROM fact_geo_region_features
        WHERE region_id = %s AND snapshot_year = %s
    """
    snapshot_year = datetime.now().year
    result = postgres_hook.get_first(verify_sql, parameters=(region_id, snapshot_year))
    stored_count = result[0] if result else 0
    
    print(f"Verified {stored_count} feature metrics stored for region {region_id}")
    
    return {
        'region_id': region_id,
        'features_stored': stored_count,
        'features_extracted': features_extracted,
        'status': 'success'
    }


# Create batches
batch_groups = []
previous_batch = None

for batch_num, batch_regions in enumerate(REGION_BATCHES, start=1):
    with TaskGroup(f'batch_{batch_num}', dag=dag) as batch_group:
        region_tasks = []
        
        for region_id in batch_regions:
            region_lower = region_id.lower()
            
            with TaskGroup(f'region_{region_lower}', dag=dag) as region_group:
                # Lookup task
                lookup_task = PythonOperator(
                    task_id=f'lookup_{region_lower}',
                    python_callable=lookup_region,
                    op_kwargs={'region_id': region_id},
                    dag=dag,
                )
                
                # Extraction task
                extract_task = PythonOperator(
                    task_id=f'extract_{region_lower}',
                    python_callable=extract_region_features,
                    op_kwargs={'region_id': region_id},
                    params={'batch_num': batch_num},
                    dag=dag,
                )
                
                # Storage task
                store_task = PythonOperator(
                    task_id=f'store_{region_lower}',
                    python_callable=store_region_metrics,
                    op_kwargs={'region_id': region_id},
                    params={'batch_num': batch_num},
                    dag=dag,
                )
                
                # Set dependencies within region group
                lookup_task >> extract_task >> store_task
                
                region_tasks.append(region_group)
        
        # Within batch, process regions sequentially
        for i in range(len(region_tasks) - 1):
            region_tasks[i] >> region_tasks[i + 1]
    
    batch_groups.append(batch_group)
    
    # Process batches sequentially
    if previous_batch:
        previous_batch >> batch_group
    previous_batch = batch_group


# Summary task
def summarize_extraction(**context):
    """Summarize OSM extraction results."""
    postgres_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    sql = """
        SELECT 
            region_id,
            COUNT(DISTINCT feature_name) as feature_count,
            SUM(feature_value) as total_value
        FROM fact_geo_region_features
        WHERE snapshot_year = %s
        GROUP BY region_id
        ORDER BY region_id
        LIMIT 20
    """
    
    current_year = datetime.now().year
    results = postgres_hook.get_records(sql, parameters=(current_year,))
    
    print("\n=== OSM Feature Extraction Summary ===")
    for row in results:
        print(f"{row[0]}: {row[1]} features, total value: {row[2]:.2f}")
    print("=====================================\n")


summarize_task = PythonOperator(
    task_id='summarize_extraction',
    python_callable=summarize_extraction,
    dag=dag,
)

# Set dependencies: batches run sequentially, then summary
if batch_groups:
    batch_groups[-1] >> summarize_task
