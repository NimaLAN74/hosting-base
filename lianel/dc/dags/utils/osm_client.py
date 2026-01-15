"""
OpenStreetMap (OSM) Feature Extraction Client

This module provides utilities for extracting OSM features using Overpass API
and aggregating them to NUTS2 regions.

Features extracted:
- Energy infrastructure (power plants, generators, substations)
- Industrial areas
- Residential buildings
- Commercial buildings
- Transport infrastructure
"""

import requests
import json
import time
import logging
from typing import Dict, List, Optional, Any, Tuple
from shapely.geometry import Point, Polygon, shape
from shapely.ops import transform
import pyproj
from functools import partial

logger = logging.getLogger(__name__)

# Overpass API endpoint
OVERPASS_API_URL = "https://overpass-api.de/api/interpreter"
OVERPASS_TIMEOUT = 180  # seconds

# OSM feature queries
# Overpass QL bbox format: (south,west,north,east) = (min_lat,min_lon,max_lat,max_lon)
# Format: way["key"="value"](south,west,north,east)
# Note: (limit:N) syntax is not valid in Overpass QL - results are limited in code after fetching
OSM_FEATURE_QUERIES = {
    'power_plant': '[out:json][timeout:25];(way["power"="plant"]({{bbox}});relation["power"="plant"]({{bbox}}););out geom;',
    'power_generator': '[out:json][timeout:25];(way["power"="generator"]({{bbox}});relation["power"="generator"]({{bbox}}););out geom;',
    'power_substation': '[out:json][timeout:25];(way["power"="substation"]({{bbox}});relation["power"="substation"]({{bbox}}););out geom;',
    'industrial_area': '[out:json][timeout:25];(way["landuse"="industrial"]({{bbox}});relation["landuse"="industrial"]({{bbox}}););out geom;',
    'residential_building': '[out:json][timeout:25];(way["building"~"^(house|apartments|residential)$"]({{bbox}});relation["building"~"^(house|apartments|residential)$"]({{bbox}}););out geom;',
    'commercial_building': '[out:json][timeout:25];(way["building"~"^(commercial|retail|office)$"]({{bbox}});relation["building"~"^(commercial|retail|office)$"]({{bbox}}););out geom;',
    'railway_station': '[out:json][timeout:25];(way["railway"="station"]({{bbox}});relation["railway"="station"]({{bbox}}););out geom;',
    'airport': '[out:json][timeout:25];(way["aeroway"="aerodrome"]({{bbox}});relation["aeroway"="aerodrome"]({{bbox}}););out geom;',
}


class OSMClient:
    """
    Client for extracting OSM features via Overpass API.
    
    Usage:
        client = OSMClient()
        features = client.extract_features(bbox=(min_lon, min_lat, max_lon, max_lat))
    """
    
    def __init__(self, api_url: str = OVERPASS_API_URL, timeout: int = OVERPASS_TIMEOUT):
        """
        Initialize OSM client.
        
        Args:
            api_url: Overpass API endpoint URL
            timeout: Request timeout in seconds
        """
        self.api_url = api_url
        self.timeout = timeout
        self.session = requests.Session()
        self.session.headers.update({'User-Agent': 'Lianel-Energy-Platform/1.0'})
    
    def _make_request(self, query: str, retries: int = 5) -> Optional[Dict]:
        """
        Make request to Overpass API with rate limiting and retry logic.
        
        Args:
            query: Overpass QL query string
            retries: Number of retry attempts
            
        Returns:
            Parsed JSON response, or None if request failed
        """
        for attempt in range(retries):
            try:
                response = self.session.post(
                    self.api_url,
                    data={'data': query},
                    timeout=self.timeout
                )
                
                # Handle rate limiting (429) with longer backoff
                if response.status_code == 429:
                    wait_time = min(60, 5 * (2 ** attempt))  # Max 60 seconds
                    logger.warning(f"Rate limited (429). Waiting {wait_time}s before retry {attempt + 1}/{retries}")
                    if attempt < retries - 1:
                        time.sleep(wait_time)
                        continue
                    else:
                        logger.error("Rate limited after all retries. Consider using a different Overpass endpoint or reducing request frequency.")
                        return None
                
                # Handle gateway timeout (504) with longer backoff
                if response.status_code == 504:
                    wait_time = min(30, 3 * (2 ** attempt))
                    logger.warning(f"Gateway timeout (504). Waiting {wait_time}s before retry {attempt + 1}/{retries}")
                    if attempt < retries - 1:
                        time.sleep(wait_time)
                        continue
                    else:
                        logger.error("Gateway timeout after all retries. Query may be too large or server overloaded.")
                        return None
                
                response.raise_for_status()
                
                data = response.json()
                
                # Check for errors in response
                if 'remark' in data and 'error' in data['remark'].lower():
                    logger.warning(f"Overpass API error: {data.get('remark')}")
                    return None
                
                return data
                
            except requests.exceptions.RequestException as e:
                # Try to get response body for better error messages
                error_detail = str(e)
                status_code = None
                if hasattr(e, 'response') and e.response is not None:
                    status_code = e.response.status_code
                    try:
                        error_body = e.response.text[:500]  # First 500 chars
                        error_detail = f"{e} - Response: {error_body}"
                    except:
                        pass
                
                # Don't retry on client errors (4xx) except 429 and 504
                if status_code and 400 <= status_code < 500 and status_code not in (429, 504):
                    logger.error(f"Client error {status_code}: {error_detail}")
                    return None
                
                logger.warning(f"Overpass API request failed (attempt {attempt + 1}/{retries}): {error_detail}")
                if attempt < retries - 1:
                    wait_time = min(30, 5 * (2 ** attempt))  # Exponential backoff, max 30s
                    time.sleep(wait_time)
                else:
                    logger.error(f"Overpass API request failed after {retries} attempts. Last error: {error_detail}")
                    return None
            except json.JSONDecodeError as e:
                logger.error(f"Failed to parse Overpass JSON response: {e}")
                return None
        
        return None
    
    def extract_features(
        self,
        bbox: Tuple[float, float, float, float],
        feature_types: Optional[List[str]] = None
    ) -> Dict[str, List[Dict]]:
        """
        Extract OSM features within a bounding box.
        
        Args:
            bbox: Bounding box as (min_lon, min_lat, max_lon, max_lat)
            feature_types: List of feature types to extract (if None, extracts all)
            
        Returns:
            Dictionary mapping feature types to lists of features
        """
        min_lon, min_lat, max_lon, max_lat = bbox
        
        if feature_types is None:
            feature_types = list(OSM_FEATURE_QUERIES.keys())
        
        results = {}
        
        for feature_type in feature_types:
            if feature_type not in OSM_FEATURE_QUERIES:
                logger.warning(f"Unknown feature type: {feature_type}")
                continue
            
            query_template = OSM_FEATURE_QUERIES[feature_type]
            # Overpass QL bbox format: (south,west,north,east) = (min_lat,min_lon,max_lat,max_lon)
            # The bbox goes directly in parentheses: way["key"="value"](south,west,north,east)
            bbox_str = f'{min_lat},{min_lon},{max_lat},{max_lon}'
            query = query_template.replace('{{bbox}}', bbox_str)
            
            logger.info(f"Extracting {feature_type} features from OSM (bbox: {bbox_str})...")
            logger.debug(f"Overpass query (first 300 chars): {query[:300]}")
            data = self._make_request(query)
            
            # If request failed, log the full query for debugging
            if data is None:
                logger.error(f"Failed query for {feature_type}: {query[:500]}")
            
            if data is None:
                results[feature_type] = []
                continue
            
            # Parse features from response - limit to prevent memory issues
            features = []
            max_features = 2000  # Hard limit per feature type to prevent OOM
            if 'elements' in data:
                for element in data['elements']:
                    if len(features) >= max_features:
                        logger.warning(f"Reached feature limit ({max_features}) for {feature_type}, truncating results")
                        break
                    if element['type'] == 'way' and 'geometry' in element:
                        # Store minimal data - only what's needed for metrics
                        features.append({
                            'id': element.get('id'),
                            'type': feature_type,
                            'geometry': element['geometry'],
                            'tags': element.get('tags', {}),
                        })
            
            results[feature_type] = features
            if len(features) >= max_features:
                logger.warning(f"Extracted {len(features)} {feature_type} features (truncated at {max_features})")
            else:
                logger.info(f"Extracted {len(features)} {feature_type} features")
            
            # Rate limiting: wait between feature type requests to avoid hitting rate limits
            # Longer delay if request failed (may have been rate limited)
            if data is None:
                time.sleep(10)  # Longer wait after failed request
            else:
                time.sleep(3)  # Standard delay between successful requests
        
        return results
    
    def calculate_feature_metrics(
        self,
        features: List[Dict],
        region_area_km2: float
    ) -> Dict[str, float]:
        """
        Calculate feature metrics (counts, areas, densities).
        
        Args:
            features: List of feature dictionaries with geometry
            region_area_km2: Area of the region in km²
            
        Returns:
            Dictionary with metrics (count, area_km2, density_per_km2)
        """
        count = len(features)
        
        # Calculate total area (for polygon features)
        total_area_m2 = 0.0
        for feature in features:
            if 'geometry' in feature:
                geom = feature['geometry']
                if geom and len(geom) > 0:
                    # Simple area calculation for polygons
                    # In production, use proper geometry library
                    try:
                        # Convert to Shapely geometry and calculate area
                        coords = []
                        for point in geom:
                            if 'lat' in point and 'lon' in point:
                                coords.append((point['lon'], point['lat']))
                        
                        if len(coords) >= 3:
                            # Create polygon and calculate area
                            polygon = Polygon(coords)
                            # Project to appropriate CRS for area calculation
                            # Using WGS84 for now (approximate)
                            total_area_m2 += polygon.area * 111000 * 111000  # Rough conversion
                    except Exception as e:
                        logger.debug(f"Error calculating area: {e}")
                        pass
        
        area_km2 = total_area_m2 / 1_000_000
        density_per_km2 = count / region_area_km2 if region_area_km2 > 0 else 0
        
        return {
            'count': count,
            'area_km2': area_km2,
            'density_per_km2': density_per_km2
        }


def get_region_bbox(region_geometry: Any) -> Tuple[float, float, float, float]:
    """
    Get bounding box from region geometry in WGS84 (lat/lon).
    
    Args:
        region_geometry: Region geometry (GeoJSON or Shapely geometry)
        
    Returns:
        Bounding box as (min_lon, min_lat, max_lon, max_lat) in WGS84
    """
    if isinstance(region_geometry, dict):
        # GeoJSON format - check if it has a CRS
        if 'crs' in region_geometry:
            crs_info = region_geometry['crs']
            # If it's not WGS84, we need to transform
            if 'properties' in crs_info and 'name' in crs_info['properties']:
                crs_name = crs_info['properties']['name']
                if 'EPSG:4326' not in crs_name and 'CRS84' not in crs_name:
                    # Need to transform from source CRS to WGS84
                    source_crs = pyproj.CRS.from_string(crs_name)
                    target_crs = pyproj.CRS.from_epsg(4326)
                    project = pyproj.Transformer.from_crs(source_crs, target_crs, always_xy=True).transform
                    geom = transform(project, shape(region_geometry))
                else:
                    geom = shape(region_geometry)
            else:
                geom = shape(region_geometry)
        else:
            # Assume WGS84 if no CRS specified
            geom = shape(region_geometry)
    else:
        geom = region_geometry
    
    # Get bounds - need to check SRID from PostGIS
    # PostGIS geometries from dim_region are likely in EPSG:3035 (ETRS89/LAEA Europe)
    # We need to transform to WGS84 (EPSG:4326) for Overpass API
    
    # Check if coordinates look like lat/lon (should be -180 to 180 for lon, -90 to 90 for lat)
    bounds = geom.bounds  # (min_x, min_y, max_x, max_y)
    min_x, min_y, max_x, max_y = bounds
    
    # If coordinates are outside lat/lon ranges, transform from projected CRS
    if abs(min_x) > 180 or abs(max_x) > 180 or abs(min_y) > 90 or abs(max_y) > 90:
        try:
            # Common CRS for EU NUTS regions: EPSG:3035 (ETRS89/LAEA Europe)
            # Transform to WGS84 (EPSG:4326)
            source_crs = pyproj.CRS.from_epsg(3035)
            target_crs = pyproj.CRS.from_epsg(4326)
            # always_xy=True means input/output is (x, y) = (lon, lat)
            project = pyproj.Transformer.from_crs(source_crs, target_crs, always_xy=True).transform
            geom_wgs84 = transform(project, geom)
            bounds = geom_wgs84.bounds
            logger.info(f"Transformed bbox from EPSG:3035 to WGS84: ({bounds[0]:.6f}, {bounds[1]:.6f}, {bounds[2]:.6f}, {bounds[3]:.6f})")
        except Exception as e:
            logger.error(f"Could not transform bbox from projected CRS: {e}")
            # If transformation fails, try to use bounds as-is (might work for some cases)
            logger.warning(f"Using bounds as-is (may cause Overpass API errors): {bounds}")
    
    # Return as (min_lon, min_lat, max_lon, max_lat) for Overpass API
    return (bounds[0], bounds[1], bounds[2], bounds[3])
