# D15 — Geo-Enrichment Dataset Specification  
Version 1.0

## 1. Purpose
Specify combined dataset linking energy metrics and OSM-based spatial features per region.

## 2. Key Fields
- `region_id`, `country_code`
- Annual energy indicators (e.g. total_final_energy_gwh)
- Electricity indicators (e.g. average_load_mw, peak_load_mw)
- OSM-based features: building counts, industrial area, renewable asset counts
- Derived densities and ratios

## 3. Use Cases
- Regional segmentation and clustering
- Spatial feature augmentation for forecasting models
- Policy and planning analysis at NUTS2 level.
