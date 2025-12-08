# D13 — Forecasting Dataset Specification  
Version 1.0

## 1. Use Case
Predict short- and medium-term electricity load or generation per country or region.

## 2. Targets
- `load_mw`
- `generation_mw` (optionally per production_type)

## 3. Features (examples)
- Time-based features: hour, day-of-week, month, is_weekend, holiday_flag
- Lagged values: previous 1–24 hours, previous 7 days
- Rolling statistics: 24h mean, 7d mean
- Regional features: building_density, industrial_density, renewable_assets_per_km2
- Macro features (optional): annual energy indicators from Eurostat

## 4. Granularity
- Base: hourly time series
- Aggregations: daily or weekly available by summation/averaging.
