# D3 — OpenStreetMap Feature Catalogue  
Version 1.0

## 1. Purpose
Define OSM features used to create geospatial context for energy and sustainability analysis.

## 2. Feature Groups
- Energy infrastructure: `power=plant`, `power=generator`, `power=substation`, `power=line`
- Industrial activity: `landuse=industrial`, `man_made=works`
- Residential and buildings: `building=house`, `building=apartments`, `building=residential`
- Commercial & services: `amenity=*`, `office=*`, `shop=*`
- Transport: `railway=station`, `aeroway=aerodrome`, major `highway=*`
- Land use / land cover: `landuse=*`, `natural=*`

## 3. Aggregation Rules
- Extract via Overpass API or PBF
- Spatially join to NUTS2 regions
- Compute counts, areas and densities by feature type

## 4. Limitations
- Tagging completeness differs by country/region
- OSM reflects “current” reality at snapshot time (no full history)
