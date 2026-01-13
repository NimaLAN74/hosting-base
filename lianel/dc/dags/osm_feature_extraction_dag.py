"""
OSM Feature Extraction DAG

This DAG extracts OpenStreetMap features and aggregates them to NUTS2 regions.
Features include:
- Energy infrastructure (power plants, generators, substations)
- Industrial areas
- Residential and commercial buildings
- Transport infrastructure

Features are aggregated per region and stored in fact_geo_region_features.
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
    'execution_timeout': timedelta(hours=6),
}

dag = DAG(
    'osm_feature_extraction',
    default_args=default_args,
    description='Extract OSM features and aggregate to NUTS2 regions',
    schedule='0 4 * * 0',  # Weekly on Sunday at 04:00 UTC
    catchup=False,
    tags=['data-ingestion', 'osm', 'geospatial', 'features'],
    max_active_runs=1,
)

# Feature types to extract
FEATURE_TYPES = [
    'power_plant',
    'power_generator',
    'power_substation',
    'industrial_area',
    'residential_building',
    'commercial_building',
    'railway_station',
    'airport',
]


def extract_region_features(region_id: str, **context) -> Dict[str, Any]:
    """
    Extract OSM features for a single NUTS2 region.
    
    Args:
        region_id: NUTS2 region code
    """
    postgres_hook = PostgresHook(postgres_conn_id='postgres_default')
    osm_client = OSMClient()
    
    # Get region geometry and area
    sql = """
        SELECT region_id, area_km2, ST_AsGeoJSON(geometry) as geometry_json
        FROM dim_region
        WHERE region_id = %s AND nuts_level = 2
    """
    
    try:
        result = postgres_hook.get_first(sql, parameters=(region_id,))
        if not result:
            print(f"Region {region_id} not found or not NUTS2 level")
            return {
                'region_id': region_id,
                'features_extracted': 0,
                'status': 'region_not_found'
            }
        
        region_id_db, area_km2, geometry_json = result
        
        if not geometry_json:
            print(f"No geometry available for region {region_id}")
            return {
                'region_id': region_id,
                'features_extracted': 0,
                'status': 'no_geometry'
            }
        
        # Parse geometry
        geometry = json.loads(geometry_json)
        
        # Get bounding box
        bbox = get_region_bbox(geometry)
        
        print(f"Extracting OSM features for region {region_id} (area: {area_km2} km²)")
        
        # Extract features
        all_features = osm_client.extract_features(bbox, FEATURE_TYPES)
        
        # Calculate metrics and store
        snapshot_year = datetime.now().year
        features_stored = 0
        
        for feature_type, features in all_features.items():
            if not features:
                continue
            
            # Calculate metrics
            metrics = osm_client.calculate_feature_metrics(features, area_km2)
            
            # Store feature counts
            count_feature_name = f"{feature_type}_count"
            count_sql = """
                INSERT INTO fact_geo_region_features (
                    region_id, feature_name, feature_value, snapshot_year
                ) VALUES (%s, %s, %s, %s)
                ON CONFLICT (region_id, feature_name, snapshot_year)
                DO UPDATE SET feature_value = EXCLUDED.feature_value,
                              updated_at = CURRENT_TIMESTAMP
            """
            
            postgres_hook.run(
                count_sql,
                parameters=(region_id, count_feature_name, metrics['count'], snapshot_year)
            )
            features_stored += 1
            
            # Store area if applicable
            if metrics['area_km2'] > 0:
                area_feature_name = f"{feature_type}_area_km2"
                area_sql = """
                    INSERT INTO fact_geo_region_features (
                        region_id, feature_name, feature_value, snapshot_year
                    ) VALUES (%s, %s, %s, %s)
                    ON CONFLICT (region_id, feature_name, snapshot_year)
                    DO UPDATE SET feature_value = EXCLUDED.feature_value,
                                  updated_at = CURRENT_TIMESTAMP
                """
                
                postgres_hook.run(
                    area_sql,
                    parameters=(region_id, area_feature_name, metrics['area_km2'], snapshot_year)
                )
                features_stored += 1
            
            # Store density
            density_feature_name = f"{feature_type}_density_per_km2"
            density_sql = """
                INSERT INTO fact_geo_region_features (
                    region_id, feature_name, feature_value, snapshot_year
                ) VALUES (%s, %s, %s, %s)
                ON CONFLICT (region_id, feature_name, snapshot_year)
                DO UPDATE SET feature_value = EXCLUDED.feature_value,
                              updated_at = CURRENT_TIMESTAMP
            """
            
            postgres_hook.run(
                density_sql,
                parameters=(region_id, density_feature_name, metrics['density_per_km2'], snapshot_year)
            )
            features_stored += 1
        
        # Log extraction
        log_sql = """
            INSERT INTO meta_osm_extraction_log (
                extraction_date, region_id, feature_types, features_extracted, status
            ) VALUES (%s, %s, %s, %s, %s)
        """
        
        feature_types_array = list(all_features.keys())
        
        postgres_hook.run(
            log_sql,
            parameters=(
                datetime.now().date(),
                region_id,
                feature_types_array,
                features_stored,
                'success' if features_stored > 0 else 'no_features'
            )
        )
        
        print(f"Successfully extracted {features_stored} feature metrics for region {region_id}")
        
        return {
            'region_id': region_id,
            'features_extracted': features_stored,
            'status': 'success'
        }
        
    except Exception as e:
        print(f"Error extracting OSM features for region {region_id}: {e}")
        
        # Log failure
        try:
            log_sql = """
                INSERT INTO meta_osm_extraction_log (
                    extraction_date, region_id, feature_types, features_extracted, status, error_message
                ) VALUES (%s, %s, %s, %s, %s, %s)
            """
            
            postgres_hook.run(
                log_sql,
                parameters=(
                    datetime.now().date(),
                    region_id,
                    [],
                    0,
                    'failed',
                    str(e)[:500]
                )
            )
        except Exception as log_error:
            print(f"Error logging failure: {log_error}")
        
        raise AirflowException(f"Failed to extract OSM features for region {region_id}: {e}")


def get_nuts2_regions(**context) -> List[str]:
    """Get list of NUTS2 regions to process."""
    postgres_hook = PostgresHook(postgres_conn_id='postgres_default')
    
    sql = """
        SELECT region_id
        FROM dim_region
        WHERE nuts_level = 2
        ORDER BY region_id
    """
    
    results = postgres_hook.get_records(sql)
    return [row[0] for row in results]


# Get regions task
get_regions_task = PythonOperator(
    task_id='get_nuts2_regions',
    python_callable=get_nuts2_regions,
    dag=dag,
)

# Create task group for region processing
with TaskGroup('extract_region_features', dag=dag) as extract_group:
    # We'll dynamically create tasks for each region
    # For now, we'll process a sample of regions (can be expanded)
    sample_regions = [
        'SE11', 'SE12', 'SE21', 'SE22', 'SE23',  # Sweden
        'DE11', 'DE12', 'DE21', 'DE22', 'DE30',  # Germany
        'FR10', 'FR21', 'FR22', 'FR30', 'FR41',  # France
    ]
    
    region_tasks = []
    for region_id in sample_regions:
        task = PythonOperator(
            task_id=f'extract_{region_id.lower()}',
            python_callable=extract_region_features,
            op_kwargs={'region_id': region_id},
            dag=dag,
        )
        region_tasks.append(task)

# Summary task
def summarize_extraction(**context):
    """Summarize OSM extraction results."""
    postgres_hook = PostgresHook(postgres_conn_id='postgres_default')
    
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

# Set dependencies
get_regions_task >> extract_group >> summarize_task
