"""
OSM Feature Extraction DAG

This DAG extracts OpenStreetMap features and aggregates them to NUTS2 regions.
Features include:
- Energy infrastructure (power plants, generators, substations)
- Industrial areas
- Residential and commercial buildings
- Transport infrastructure

Features are aggregated per region and stored in fact_geo_region_features.

Subtasks:
- Region lookup: Get region geometry
- Feature extraction: Extract features from OSM
- Metric storage: Calculate and store metrics
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


def get_nuts2_regions(**context) -> List[str]:
    """Get list of NUTS2 regions to process."""
    postgres_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
    sql = """
        SELECT region_id
        FROM dim_region
        WHERE nuts_level = 2
        ORDER BY region_id
    """
    
    results = postgres_hook.get_records(sql)
    regions = [row[0] for row in results]
    
    return regions


def lookup_region(region_id: str, **context) -> Dict[str, Any]:
    """Get region geometry and metadata."""
    postgres_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    
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
                'status': 'region_not_found'
            }
        
        region_id_db, area_km2, geometry_json = result
        
        if not geometry_json:
            print(f"No geometry available for region {region_id}")
            return {
                'region_id': region_id,
                'status': 'no_geometry'
            }
        
        return {
            'region_id': region_id,
            'area_km2': float(area_km2) if area_km2 else 0.0,
            'status': 'success'
        }
        
    except Exception as e:
        print(f"Error looking up region {region_id}: {e}")
        context['ti'].xcom_push(key=f'region_data_{region_id}', value=None)
        raise AirflowException(f"Failed to lookup region {region_id}: {e}")


def extract_region_features(region_id: str, **context) -> Dict[str, Any]:
    """Extract OSM features for a region."""
    ti = context['ti']
    task_ids = [f'region_{region_id.lower()}.lookup_{region_id.lower()}']
    lookup_result = ti.xcom_pull(task_ids=task_ids)
    
    if not lookup_result or not isinstance(lookup_result, dict):
        return {
            'region_id': region_id,
            'features_extracted': 0,
            'status': 'region_not_found'
        }
    
    # Get region data from lookup task
    postgres_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    sql = """
        SELECT region_id, area_km2, ST_AsGeoJSON(geometry) as geometry_json
        FROM dim_region
        WHERE region_id = %s AND nuts_level = 2
    """
    result = postgres_hook.get_first(sql, parameters=(region_id,))
    
    if not result or not result[2]:
        return {
            'region_id': region_id,
            'features_extracted': 0,
            'status': 'no_geometry'
        }
    
    osm_client = OSMClient()
    
    try:
        # Parse geometry
        geometry = json.loads(result[2])
        area_km2 = float(result[1]) if result[1] else 0.0
        
        # Get bounding box
        bbox = get_region_bbox(geometry)
        
        print(f"Extracting OSM features for region {region_id} (area: {area_km2} km²)")
        
        # Extract features
        all_features = osm_client.extract_features(bbox, FEATURE_TYPES)
        
        feature_count = sum(len(features) for features in all_features.values())
        
        return {
            'region_id': region_id,
            'features': all_features,
            'area_km2': area_km2,
            'feature_count': feature_count,
            'feature_types': list(all_features.keys()),
            'status': 'success'
        }
        
    except Exception as e:
        print(f"Error extracting OSM features for region {region_id}: {e}")
        raise AirflowException(f"Failed to extract OSM features for region {region_id}: {e}")


def store_region_metrics(region_id: str, **context) -> Dict[str, Any]:
    """Calculate and store metrics for extracted features."""
    postgres_hook = PostgresHook(postgres_conn_id='lianel_energy_db')
    ti = context['ti']
    
    task_ids = [f'region_{region_id.lower()}.extract_{region_id.lower()}']
    extract_result = ti.xcom_pull(task_ids=task_ids)
    
    if not extract_result or not isinstance(extract_result, dict):
        return {
            'region_id': region_id,
            'features_stored': 0,
            'status': 'no_features'
        }
    
    all_features = extract_result.get('features', {})
    area_km2 = extract_result.get('area_km2', 0.0)
    
    if not all_features:
        return {
            'region_id': region_id,
            'features_stored': 0,
            'status': 'no_features'
        }
    
    osm_client = OSMClient()
    snapshot_year = datetime.now().year
    features_stored = 0
    
    try:
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
        
        print(f"Successfully stored {features_stored} feature metrics for region {region_id}")
        
        return {
            'region_id': region_id,
            'features_stored': features_stored,
            'status': 'success'
        }
        
    except Exception as e:
        print(f"Error storing metrics for region {region_id}: {e}")
        
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
        
        raise AirflowException(f"Failed to store metrics for region {region_id}: {e}")


# Get regions task
get_regions_task = PythonOperator(
    task_id='get_nuts2_regions',
    python_callable=get_nuts2_regions,
    dag=dag,
)

# Sample regions (can be expanded to all NUTS2 regions)
sample_regions = [
    'SE11', 'SE12', 'SE21', 'SE22', 'SE23',  # Sweden
    'DE11', 'DE12', 'DE21', 'DE22', 'DE30',  # Germany
    'FR10', 'FR21', 'FR22', 'FR30', 'FR41',  # France
]

# Create task groups for each region
region_groups = []

for region_id in sample_regions:
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
            dag=dag,
        )
        
        # Storage task
        store_task = PythonOperator(
            task_id=f'store_{region_lower}',
            python_callable=store_region_metrics,
            op_kwargs={'region_id': region_id},
            dag=dag,
        )
        
        # Set dependencies within region group
        lookup_task >> extract_task >> store_task
    
    region_groups.append(region_group)


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

# Set dependencies: get regions, then all region groups run in parallel, then summary
for region_group in region_groups:
    get_regions_task >> region_group >> summarize_task