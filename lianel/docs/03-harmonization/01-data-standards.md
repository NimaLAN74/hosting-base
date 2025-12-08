# D5 — Data Standards  
Version 1.0

## 1. Purpose
Define common standards for identifiers, time, units, naming, spatial representation and metadata.

## 2. Identifiers
- Countries: ISO 3166-1 alpha-2
- Regions: NUTS0/1/2 codes
- Energy products & flows: Eurostat code lists

## 3. Time & Resolution
- All timestamps in UTC, ISO 8601 format
- Supported resolutions: annual, daily, hourly, 15-minute

## 4. Units
- Energy: GWh
- Power: MW
- Area: km²

## 5. Naming Conventions
- snake_case for fields
- `dim_*`, `fact_*`, `meta_*` prefix for tables

## 6. Spatial Standards
- EPSG:3035 for metrics and area
- EPSG:4326 for external interoperability

## 7. Metadata
Each dataset must at minimum record:
- `src_system`
- `src_location` (API or file)
- `extraction_date`
- `original_unit`
- `harmonisation_version`
