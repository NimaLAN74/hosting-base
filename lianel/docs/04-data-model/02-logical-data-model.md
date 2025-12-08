# D10 — Logical Data Model  
Version 1.0

## 1. Fact Tables
- `fact_energy_annual(country_code, year, sector, product, flow, value_gwh, unit)`
- `fact_electricity_timeseries(timestamp_utc, country_code, production_type, load_mw, generation_mw)`
- `fact_geo_region_features(region_id, feature_name, feature_value, snapshot_year)`

## 2. Dimension Tables
- `dim_country(country_code, country_name, nuts0_code)`
- `dim_region(region_id, nuts_level, country_code, name, area_km2, geometry_ref)`
- `dim_energy_product(product_code, product_name, category)`
- `dim_production_type(code, name, renewable_flag)`
