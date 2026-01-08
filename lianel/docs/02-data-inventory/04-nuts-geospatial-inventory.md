# D4 — NUTS Geospatial Inventory  
Version 2.0  
**Last Updated**: January 7, 2026  
**Tested**: Remote Host (72.60.80.84)

## 1. Purpose
Describe NUTS-based geospatial datasets used as the spatial backbone of the platform.

## 2. NUTS Levels

| Level | Description | Features | Use Case |
|-------|-------------|----------|----------|
| **NUTS0** | Country level | 37 features | Country-level energy data mapping |
| **NUTS1** | Major socio-economic regions | 125 features | Regional analysis |
| **NUTS2** | Main analysis and ML unit | 334 features | **Primary ML unit** for clustering and forecasting |

## 3. Data Source: Eurostat GISCO

### 3.1 Download URLs

**Base URL**: `https://gisco-services.ec.europa.eu/distribution/v2/nuts/geojson/`

**Files (2021 version, 1:1M scale, GeoJSON format)**:

| Level | File Name | URL | Size | Features |
|-------|-----------|-----|------|----------|
| NUTS0 | `NUTS_RG_01M_2021_4326_LEVL_0.geojson` | [Download](https://gisco-services.ec.europa.eu/distribution/v2/nuts/geojson/NUTS_RG_01M_2021_4326_LEVL_0.geojson) | 10.93 MB | 37 |
| NUTS1 | `NUTS_RG_01M_2021_4326_LEVL_1.geojson` | [Download](https://gisco-services.ec.europa.eu/distribution/v2/nuts/geojson/NUTS_RG_01M_2021_4326_LEVL_1.geojson) | 13.77 MB | 125 |
| NUTS2 | `NUTS_RG_01M_2021_4326_LEVL_2.geojson` | [Download](https://gisco-services.ec.europa.eu/distribution/v2/nuts/geojson/NUTS_RG_01M_2021_4326_LEVL_2.geojson) | 17.18 MB | 334 |

**Total Size**: ~42 MB (all three levels)

### 3.2 File Format Details

- **Format**: GeoJSON
- **Scale**: 1:1 million (suitable for analysis)
- **Version**: 2021 (latest stable version)
- **Update Frequency**: Annual (new versions released each year)
- **Coordinate System**: EPSG:4326 (WGS84) - will be transformed to EPSG:3035

### 3.3 Alternative Formats

GISCO also provides:
- **Shapefile** format (`.shp`)
- **GeoPackage** format (`.gpkg`)
- **10M scale** versions (smaller, for visualization only)

**Recommendation**: Use GeoJSON for easier Python/GeoPandas processing.

## 4. Data Structure

### 4.1 Column Schema

All NUTS levels share the same column structure:

| Column | Type | Description | Example |
|--------|------|-------------|---------|
| `NUTS_ID` | String | Unique NUTS identifier | `"SE11"`, `"DE12"` |
| `LEVL_CODE` | Integer | NUTS level (0, 1, or 2) | `0`, `1`, `2` |
| `CNTR_CODE` | String | ISO 2-letter country code | `"SE"`, `"DE"`, `"FR"` |
| `NAME_LATN` | String | Name in Latin script | `"Stockholm"`, `"Karlsruhe"` |
| `NUTS_NAME` | String | Official NUTS name | `"Stockholm"`, `"Karlsruhe"` |
| `MOUNT_TYPE` | String | Mountain type classification | Various |
| `URBN_TYPE` | String | Urban type classification | Various |
| `COAST_TYPE` | String | Coastal type classification | Various |
| `geometry` | Geometry | Polygon/MultiPolygon geometry | GeoJSON geometry |

### 4.2 Sample Data

**NUTS0 (Country Level)**:
- `AL`: Shqipëria (Albania)
- `CZ`: Česko (Czech Republic)
- `DE`: Deutschland (Germany)
- `SE`: Sverige (Sweden)
- ... (37 total countries/regions)

**NUTS1 (Regional Level)**:
- `DEA`: Nordrhein-Westfalen (Germany)
- `CH0`: Schweiz/Suisse/Svizzera (Switzerland)
- `DE7`: Hessen (Germany)
- ... (125 total regions)

**NUTS2 (Sub-Regional Level)**:
- `DE12`: Karlsruhe (Germany)
- `CH05`: Ostschweiz (Switzerland)
- `CZ02`: Střední Čechy (Czech Republic)
- ... (334 total regions)

## 5. Geometry & Coordinate Systems

### 5.1 Geometry Types
- **Type**: POLYGON or MULTIPOLYGON
- **Complexity**: 1:1M scale provides good balance of detail and file size

### 5.2 Coordinate Reference Systems

**Source CRS (EPSG:4326 - WGS84)**:
- Geographic coordinate system
- Used for download and initial processing
- Standard for web mapping and interoperability

**Target CRS (EPSG:3035 - ETRS89 / LAEA Europe)**:
- Lambert Azimuthal Equal Area projection
- **Used for analysis** (area calculations, spatial joins)
- Preserves area measurements (important for energy density calculations)
- Covers Europe effectively

### 5.3 Coordinate Transformation

**Python/GeoPandas Example**:
```python
import geopandas as gpd

# Load from GeoJSON (EPSG:4326)
gdf = gpd.read_file("NUTS_RG_01M_2021_4326_LEVL_2.geojson")
print(f"Source CRS: {gdf.crs}")  # EPSG:4326

# Transform to EPSG:3035 for analysis
gdf_proj = gdf.to_crs("EPSG:3035")
print(f"Projected CRS: {gdf_proj.crs}")  # EPSG:3035

# Calculate area in km²
gdf_proj['area_km2'] = gdf_proj.geometry.area / 1_000_000
```

## 6. Spatial Statistics

### 6.1 Area Statistics (EPSG:3035)

**NUTS0 (Countries)**:
- Min area: 160.03 km²
- Max area: 780,372.97 km²
- Mean area: 160,164.40 km²
- Total area: 5,926,082.74 km²

**NUTS1 (Regions)**:
- Min area: 160.03 km²
- Max area: 386,793.71 km²
- Mean area: ~47,000 km²

**NUTS2 (Sub-Regions)**:
- Min area: 14.01 km²
- Max area: 227,089.77 km²
- Mean area: ~17,700 km²

### 6.2 Geographic Coverage

**Bounds (EPSG:4326)**:
- West: -63.15° (includes overseas territories)
- East: 55.84°
- South: -21.39°
- North: 80.83°

**Coverage**: EU27 + EFTA countries + candidate countries

## 7. Data Loading & Processing

### 7.1 Download Script

**Python Example**:
```python
import requests
from pathlib import Path

BASE_URL = "https://gisco-services.ec.europa.eu/distribution/v2/nuts/geojson/"
files = {
    "NUTS0": "NUTS_RG_01M_2021_4326_LEVL_0.geojson",
    "NUTS1": "NUTS_RG_01M_2021_4326_LEVL_1.geojson",
    "NUTS2": "NUTS_RG_01M_2021_4326_LEVL_2.geojson",
}

for level, filename in files.items():
    url = BASE_URL + filename
    response = requests.get(url, stream=True)
    
    with open(filename, 'wb') as f:
        for chunk in response.iter_content(chunk_size=8192):
            f.write(chunk)
    
    print(f"Downloaded {level}: {filename}")
```

### 7.2 Loading with GeoPandas

```python
import geopandas as gpd

# Load NUTS2 (primary analysis level)
gdf = gpd.read_file("NUTS_RG_01M_2021_4326_LEVL_2.geojson")

# Check structure
print(f"Features: {len(gdf)}")
print(f"Columns: {gdf.columns.tolist()}")
print(f"CRS: {gdf.crs}")

# Transform to analysis CRS
gdf_proj = gdf.to_crs("EPSG:3035")

# Calculate area
gdf_proj['area_km2'] = gdf_proj.geometry.area / 1_000_000

# Filter for EU27 (example: Sweden)
gdf_se = gdf_proj[gdf_proj['CNTR_CODE'] == 'SE']
print(f"Sweden NUTS2 regions: {len(gdf_se)}")
```

### 7.3 Validation & Quality Checks

**Recommended Checks**:
1. **Geometry Validation**: Ensure all geometries are valid
   ```python
   invalid = gdf[~gdf.geometry.is_valid]
   if len(invalid) > 0:
       print(f"Warning: {len(invalid)} invalid geometries")
   ```

2. **NUTS Hierarchy**: Verify NUTS1 regions contain NUTS2 regions
   ```python
   # Spatial join to verify hierarchy
   nuts1 = gpd.read_file("NUTS_RG_01M_2021_4326_LEVL_1.geojson")
   nuts2 = gpd.read_file("NUTS_RG_01M_2021_4326_LEVL_2.geojson")
   
   # Check containment
   joined = gpd.sjoin(nuts2, nuts1, how="left", predicate="within")
   ```

3. **Country Code Consistency**: Verify CNTR_CODE matches NUTS_ID prefix
   ```python
   # NUTS2 should start with country code
   gdf['nuts_country'] = gdf['NUTS_ID'].str[:2]
   mismatches = gdf[gdf['CNTR_CODE'] != gdf['nuts_country']]
   ```

## 8. Integration with Other Data Sources

### 8.1 Eurostat Energy Data
- **Mapping**: Eurostat `geo` dimension → NUTS0 `CNTR_CODE`
- **Example**: `geo="SE"` → `CNTR_CODE="SE"` in NUTS0
- **Use Case**: Join energy data with country boundaries

### 8.2 ENTSO-E (Future)
- **Mapping**: ENTSO-E bidding zones → approximate NUTS1/NUTS2 regions
- **Challenge**: Bidding zones don't align perfectly with NUTS boundaries
- **Solution**: Spatial overlay or manual mapping table

### 8.3 OSM (Future)
- **Mapping**: OSM features → aggregate by NUTS2 regions
- **Use Case**: Calculate energy infrastructure density per NUTS2 region
- **Method**: Spatial join OSM points/polygons with NUTS2 boundaries

## 9. Storage in Database

### 9.1 PostgreSQL/PostGIS Schema

**Table**: `dim_region`

```sql
CREATE TABLE dim_region (
    nuts_id VARCHAR(10) PRIMARY KEY,
    level_code INTEGER NOT NULL,
    cntr_code VARCHAR(2) NOT NULL,
    name_latn VARCHAR(255),
    nuts_name VARCHAR(255),
    mount_type VARCHAR(50),
    urbn_type VARCHAR(50),
    coast_type VARCHAR(50),
    geometry GEOMETRY(MULTIPOLYGON, 3035),  -- EPSG:3035 for analysis
    geometry_wgs84 GEOMETRY(MULTIPOLYGON, 4326),  -- EPSG:4326 for display
    area_km2 DOUBLE PRECISION,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Spatial index for fast queries
CREATE INDEX idx_dim_region_geometry ON dim_region USING GIST (geometry);
CREATE INDEX idx_dim_region_cntr_code ON dim_region (cntr_code);
CREATE INDEX idx_dim_region_level ON dim_region (level_code);
```

### 9.2 Loading into PostGIS

```python
from sqlalchemy import create_engine
import geopandas as gpd

# Load and transform
gdf = gpd.read_file("NUTS_RG_01M_2021_4326_LEVL_2.geojson")
gdf_proj = gdf.to_crs("EPSG:3035")
gdf_proj['area_km2'] = gdf_proj.geometry.area / 1_000_000

# Rename columns to match schema
gdf_proj = gdf_proj.rename(columns={
    'NUTS_ID': 'nuts_id',
    'LEVL_CODE': 'level_code',
    'CNTR_CODE': 'cntr_code',
    'NAME_LATN': 'name_latn',
    'NUTS_NAME': 'nuts_name',
})

# Connect to PostgreSQL
engine = create_engine("postgresql://user:pass@host/db")

# Write to PostGIS
gdf_proj.to_postgis("dim_region", engine, if_exists="replace", index=False)
```

## 10. Best Practices

### 10.1 Download & Update Strategy
- **Initial Load**: Download all three levels
- **Updates**: Check annually for new versions (typically released in Q1)
- **Versioning**: Keep track of NUTS version (currently 2021)
- **Storage**: Store original GeoJSON files for reference

### 10.2 Processing Recommendations
1. **Always transform to EPSG:3035** for analysis (area calculations, spatial joins)
2. **Keep EPSG:4326** for display/mapping (web maps, visualization)
3. **Calculate area** after transformation (EPSG:3035 preserves area)
4. **Validate geometries** before loading into database
5. **Create spatial indexes** for performance

### 10.3 Performance Considerations
- **File Size**: 1:1M scale is good balance (42 MB total)
- **Feature Count**: 334 NUTS2 regions is manageable
- **Spatial Indexes**: Essential for fast spatial queries
- **Caching**: Consider caching transformed geometries

## 11. Test Results

**Test Date**: January 7, 2026  
**Test Location**: Remote Host (72.60.80.84)  
**Test Script**: `/root/lianel/dc/scripts/test-nuts-gisco.py`

### 11.1 Download & Load Tests
- ✅ **NUTS0**: Downloaded (10.93 MB), loaded (37 features)
- ✅ **NUTS1**: Downloaded (13.77 MB), loaded (125 features)
- ✅ **NUTS2**: Downloaded (17.18 MB), loaded (334 features)

### 11.2 Transformation Tests
- ✅ All levels successfully transformed to EPSG:3035
- ✅ Area calculations verified
- ✅ Sample features validated

### 11.3 Test Artifacts
- **Test Results**: `/root/lianel/dc/data/samples/nuts/nuts-test-results.json`
- **Downloaded Files**: `/root/lianel/dc/data/samples/nuts/*.geojson`
- **Test Script**: `/root/lianel/dc/scripts/test-nuts-gisco.py`

## 12. References

- **GISCO Portal**: https://ec.europa.eu/eurostat/web/gisco/geodata
- **NUTS Download**: https://ec.europa.eu/eurostat/web/gisco/geodata/reference-data/administrative-units-statistical-units/nuts
- **NUTS Regulation**: https://ec.europa.eu/eurostat/web/nuts/background
- **EPSG:3035**: https://epsg.io/3035
- **EPSG:4326**: https://epsg.io/4326
- **GeoPandas Documentation**: https://geopandas.org/
