#!/usr/bin/env python3
"""
Load NUTS boundaries into dim_region table
"""
import geopandas as gpd
from sqlalchemy import create_engine
import subprocess
import os

# Get database password
try:
    result = subprocess.run(['grep', 'POSTGRES_PASSWORD', '.env'], 
                          capture_output=True, text=True, cwd='/root/lianel/dc')
    password = result.stdout.split('=')[1].strip() if result.returncode == 0 else 'postgres'
except:
    password = 'postgres'

# Database connection
conn_string = f'postgresql://postgres:{password}@172.18.0.1:5432/lianel_energy'
print('Connecting to database...')
engine = create_engine(conn_string)

# Load NUTS2 data
nuts_file = '/root/lianel/dc/data/samples/nuts/NUTS_RG_01M_2021_4326_LEVL_2.geojson'
print(f'Loading {nuts_file}...')
gdf = gpd.read_file(nuts_file)
print(f'Loaded {len(gdf)} regions')

# Transform to EPSG:3035
gdf_proj = gdf.to_crs('EPSG:3035')
gdf_proj['area_km2'] = gdf_proj.geometry.area / 1_000_000

# Keep WGS84 geometry
gdf_proj['geometry_wgs84'] = gdf.geometry

# Rename columns
gdf_proj = gdf_proj.rename(columns={
    'NUTS_ID': 'region_id',
    'LEVL_CODE': 'level_code',
    'CNTR_CODE': 'cntr_code',
    'NAME_LATN': 'name_latn',
    'NUTS_NAME': 'nuts_name',
})

# Select columns for database
cols = ['region_id', 'level_code', 'cntr_code', 'name_latn', 'nuts_name',
        'MOUNT_TYPE', 'URBN_TYPE', 'COAST_TYPE', 'area_km2', 'geometry', 'geometry_wgs84']

# Write to PostGIS
print('Writing to database...')
gdf_proj[cols].to_postgis('dim_region', engine, if_exists='append', index=False)
print(f'✅ Loaded {len(gdf_proj)} NUTS2 regions to database')

