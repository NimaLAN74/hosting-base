# D4 — NUTS Geospatial Inventory  
Version 1.0

## 1. Purpose
Describe NUTS-based geospatial datasets used as the spatial backbone of the platform.

## 2. NUTS Levels
- NUTS0: country
- NUTS1: major socio-economic regions
- NUTS2: main analysis and ML unit

## 3. Sources
- Eurostat GISCO boundary datasets
- EuroGeographics for validation

## 4. Geometry & CRS
- Geometry: POLYGON/MULTIPOLYGON
- Analysis CRS: EPSG:3035 (ETRS89 / LAEA Europe)
- Interop CRS: EPSG:4326 (WGS84)

## 5. Integration
- Eurostat energy: NUTS0 mapping
- ENTSO-E: country and approximate bidding zone overlays
- OSM: feature aggregation by NUTS2 regions
