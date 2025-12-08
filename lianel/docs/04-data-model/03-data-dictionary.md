# D11 — Data Dictionary  
Version 1.0

## 1. Purpose
Provide authoritative definitions of all key fields.

## 2. Core Fields (examples)

### Country & Region
- `country_code` (string): ISO 3166-1 alpha-2
- `region_id` (string): NUTS code
- `nuts_level` (int): 0, 1 or 2

### Time
- `year` (int): calendar year
- `timestamp_utc` (datetime): UTC timestamp

### Energy
- `sector` (string): industry, transport, households, services, etc.
- `flow` (string): production, imports, exports, final consumption, etc.
- `product` (string): Eurostat product code
- `value_gwh` (float): energy as GWh

### Electricity
- `production_type` (string): technology/fuel type
- `load_mw` (float)
- `generation_mw` (float)

### Geo Features
- `feature_name` (string): e.g. 'building_residential_count'
- `feature_value` (numeric)
- `snapshot_year` (int)
